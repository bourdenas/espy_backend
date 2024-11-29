use crate::{
    api::{FirestoreApi, GogScrape, MetacriticApi, SteamDataApi, SteamScrape},
    documents::{ExternalGame, GameEntry, IgdbExternalGame, IgdbGame, Keyword, StoreName},
    library::firestore,
    logging::{DiffEvent, LogWebhooksRequest, RejectEvent},
    resolver::ResolveApi,
    Status,
};
use chrono::Utc;
use std::{convert::Infallible, sync::Arc};
use tracing::{instrument, trace_span, warn, Instrument};
use warp::http::StatusCode;

use super::{filtering::GameFilter, prefiltering::IgdbPrefilter};

#[instrument(
    name = "add_game",
    level = "info",
    skip(igdb_game, firestore, resolver, game_filter)
)]
pub async fn add_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    game_filter: Arc<GameFilter>,
) -> Result<impl warp::Reply, Infallible> {
    LogWebhooksRequest::add_game(&igdb_game);

    if !IgdbPrefilter::filter(&igdb_game) {
        RejectEvent::prefilter(IgdbPrefilter::explain(&igdb_game));
        return Ok(StatusCode::OK);
    }

    tokio::spawn(
        async move {
            handle_add_game(igdb_game, firestore, resolver, game_filter).await;
        }
        .instrument(trace_span!("spawn_add_game")),
    );

    Ok(StatusCode::OK)
}

async fn handle_add_game(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    game_filter: Arc<GameFilter>,
) {
    match resolver.resolve(igdb_game).await {
        Ok(mut game_entry) => {
            if !game_filter.apply(&game_entry) {
                RejectEvent::filter(game_filter.explain(&game_entry));
            } else if let Err(status) = firestore::games::write(&firestore, &mut game_entry).await {
                // log_event.log_error(status);
            }
        }
        Err(status) => {
            // log_event.log_error(status)
        }
    }
}

#[instrument(
    name = "update_game",
    level = "info",
    skip(igdb_game, firestore, resolver, game_filter)
)]
pub async fn update_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    game_filter: Arc<GameFilter>,
) -> Result<impl warp::Reply, Infallible> {
    LogWebhooksRequest::update_game(&igdb_game);

    if !IgdbPrefilter::filter(&igdb_game) {
        RejectEvent::prefilter(IgdbPrefilter::explain(&igdb_game));
        return Ok(StatusCode::OK);
    }

    tokio::spawn(
        async move {
            handle_update_game(igdb_game, firestore, resolver, game_filter).await;
        }
        .instrument(trace_span!("spawn_update_game")),
    );

    Ok(StatusCode::OK)
}

async fn handle_update_game(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    game_filter: Arc<GameFilter>,
) {
    match firestore::games::read(&firestore, igdb_game.id).await {
        Ok(mut game_entry) => {
            let diff = game_entry.igdb_game.diff(&igdb_game);
            DiffEvent::diff(&diff);

            if diff.needs_resolve() {
                match resolver.resolve(igdb_game).await {
                    Ok(mut game_entry) => {
                        if let Err(status) =
                            firestore::games::write(&firestore, &mut game_entry).await
                        {
                            // log_event.log_error(status)
                        }
                    }
                    Err(status) => {
                        // log_event.log_error(status)
                    }
                }
            } else if diff.is_not_empty() || needs_steam_update(&game_entry) {
                if let Err(status) = update_steam_data(firestore, &mut game_entry, igdb_game).await
                {
                    // log_event.log_error(status)
                }
            } else {
                // log ignore update
            }
        }
        Err(Status::NotFound(_)) => {
            handle_add_game(igdb_game, firestore, resolver, game_filter).await
        }
        Err(status) => {
            // log_event.log_error(status)
        }
    }
}

fn needs_steam_update(game_entry: &GameEntry) -> bool {
    let today = Utc::now().naive_utc().and_utc().timestamp();
    let close_to_release = (today - game_entry.release_date).abs() < 8 * DAY_SECS;

    // Update if never updated || was not updated in the last 7 days ago ||
    // it is close to release and was not updated last 24hrs.
    game_entry.last_updated == 0
        || today - game_entry.last_updated > 7 * DAY_SECS
        || (close_to_release && today - game_entry.last_updated > 1 * DAY_SECS)
}

const DAY_SECS: i64 = 24 * 60 * 60;

/// Refresh data from Steam.
///
/// Steam data are refreshed periodically since they are more dynamic than other
/// sources.
async fn update_steam_data(
    firestore: Arc<FirestoreApi>,
    game_entry: &mut GameEntry,
    igdb_game: IgdbGame,
) -> Result<(), Status> {
    game_entry.update_igdb(igdb_game);

    // Spawn a task to retrieve steam data.
    let steam_handle =
        match firestore::external_games::get_steam_id(&firestore, game_entry.id).await {
            Ok(steam_appid) => steam_appid.and_then(|steam_appid| {
                Some(tokio::spawn(
                    async move {
                        let steam = SteamDataApi::new();
                        steam.retrieve_steam_data(&steam_appid).await
                    }
                    .instrument(trace_span!("spawn_steam_request")),
                ))
            }),
            Err(status) => {
                warn!("{status}");
                None
            }
        };

    // Spawn a task to scrape steam user tags.
    let steam_tags_handle = match &game_entry.steam_data {
        Some(steam_data) => {
            let website = format!(
                "https://store.steampowered.com/app/{}/",
                steam_data.steam_appid
            );
            Some(tokio::spawn(
                async move { SteamScrape::scrape(&website).await }
                    .instrument(trace_span!("spawn_steam_scrape")),
            ))
        }
        None => None,
    };

    // Spawn a task to retrieve metacritic score.
    let slug = MetacriticApi::guess_id(&game_entry.igdb_game.url).to_owned();
    let metacritic_handle = tokio::spawn(
        async move { MetacriticApi::get_score(&slug).await }
            .instrument(trace_span!("spawn_metacritic_request")),
    );

    if let Some(handle) = steam_handle {
        match handle.await {
            Ok(result) => match result {
                Ok(steam_data) => game_entry.add_steam_data(steam_data),
                Err(status) => warn!("{status}"),
            },
            Err(status) => warn!("{status}"),
        }
    }

    if let Some(handle) = steam_tags_handle {
        match handle.await {
            Ok(result) => {
                if let Some(steam_scrape_data) = result {
                    if let Some(steam_data) = &mut game_entry.steam_data {
                        steam_data.user_tags = steam_scrape_data.user_tags;
                    }
                }
            }
            Err(status) => warn!("{status}"),
        }
    }

    match metacritic_handle.await {
        Ok(response) => {
            if let Some(metacritic) = response {
                game_entry
                    .scores
                    .add_metacritic(metacritic, game_entry.release_date);
            }
        }
        Err(status) => warn!("{status}"),
    }

    firestore::games::write(&firestore, game_entry).await
}

#[instrument(name = "external_game", level = "info", skip(external_game, firestore))]
pub async fn external_games_webhook(
    external_game: IgdbExternalGame,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    let mut external_game = ExternalGame::from(external_game);
    LogWebhooksRequest::external_game(&external_game);

    if matches!(external_game.store_name, StoreName::other(_)) {
        return Ok(StatusCode::OK);
    }

    tokio::spawn(
        async move {
            match external_game.store_name {
                StoreName::gog => {
                    if let Some(url) = &external_game.store_url {
                        match GogScrape::scrape(url).await {
                            Ok(gog_data) => external_game.gog_data = Some(gog_data),
                            Err(status) => warn!("GOG scraping failed: {status}"),
                        }
                    }
                }
                _ => {}
            }

            if let Err(status) = firestore::external_games::write(&firestore, &external_game).await
            {
                // event.log_error(external_game, status)
            }
        }
        .instrument(trace_span!("spawn_external_games")),
    );

    Ok(StatusCode::OK)
}

#[instrument(name = "keyword", level = "info", skip(keyword, firestore))]
pub async fn keywords_webhook(
    keyword: Keyword,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    LogWebhooksRequest::keyword(&keyword);

    if let Err(status) = firestore::keywords::write(&firestore, &keyword).await {
        //   event.log_error(status)
    }

    Ok(StatusCode::OK)
}
