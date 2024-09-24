use crate::{
    api::{FirestoreApi, GogScrape, MetacriticApi, SteamDataApi, SteamScrape},
    documents::{ExternalGame, GameEntry, IgdbExternalGame, IgdbGame, Keyword},
    library::firestore,
    resolver::ResolveApi,
    Status,
};
use chrono::Utc;
use std::{convert::Infallible, sync::Arc};
use tracing::{instrument, trace_span, warn, Instrument};
use warp::http::StatusCode;

use super::{
    event_logs::{ExternalGameEvent, KeywordsEvent, UpdateGameEvent},
    filtering::GameFilter,
    prefiltering::IgdbPrefilter,
};

#[instrument(level = "trace", skip(igdb_game, firestore, resolver, game_filter))]
pub async fn add_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    game_filter: Arc<GameFilter>,
) -> Result<impl warp::Reply, Infallible> {
    let event = UpdateGameEvent::new(
        igdb_game.id,
        igdb_game.name.clone(),
        ADD_GAME_HANDLER.to_owned(),
    );

    if !IgdbPrefilter::filter(&igdb_game) {
        event.log_prefilter_reject(IgdbPrefilter::explain(&igdb_game));
        return Ok(StatusCode::OK);
    }

    tokio::spawn(async move {
        handle_add_game(igdb_game, firestore, resolver, game_filter, event).await;
    });

    Ok(StatusCode::OK)
}

async fn handle_add_game(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    game_filter: Arc<GameFilter>,
    log_event: UpdateGameEvent,
) {
    match resolver.resolve(igdb_game).await {
        Ok(mut game_entry) => {
            if !game_filter.filter(&game_entry) {
                log_event.log_reject(game_filter.explain(&game_entry));
            } else if let Err(status) = firestore::games::write(&firestore, &mut game_entry).await {
                log_event.log_error(status);
            } else {
                log_event.log_added()
            }
        }
        Err(status) => log_event.log_error(status),
    }
}

#[instrument(level = "trace", skip(igdb_game, firestore, resolver, game_filter))]
pub async fn update_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    game_filter: Arc<GameFilter>,
) -> Result<impl warp::Reply, Infallible> {
    let event = UpdateGameEvent::new(
        igdb_game.id,
        igdb_game.name.clone(),
        UPDATE_GAME_HANDLER.to_owned(),
    );

    if !IgdbPrefilter::filter(&igdb_game) {
        event.log_prefilter_reject(IgdbPrefilter::explain(&igdb_game));
        return Ok(StatusCode::OK);
    }

    tokio::spawn(
        async move {
            handle_update_game(igdb_game, firestore, resolver, game_filter, event).await;
        }
        .instrument(trace_span!("spawn_handle_update_game")),
    );

    Ok(StatusCode::OK)
}

async fn handle_update_game(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    game_filter: Arc<GameFilter>,
    log_event: UpdateGameEvent,
) {
    match firestore::games::read(&firestore, igdb_game.id).await {
        Ok(mut game_entry) => match game_entry.igdb_game.diff(&igdb_game) {
            diff if diff.empty() => {
                if needs_steam_update(&game_entry) {
                    match update_steam_data(firestore, &mut game_entry, igdb_game).await {
                        Ok(()) => log_event.log_updated(diff),
                        Err(status) => log_event.log_error(status),
                    }
                } else {
                    log_event.log_no_update()
                }
            }
            diff if diff.needs_resolve() => match resolver.resolve(igdb_game).await {
                Ok(mut game_entry) => {
                    match firestore::games::write(&firestore, &mut game_entry).await {
                        Ok(()) => log_event.log_updated(diff),
                        Err(status) => log_event.log_error(status),
                    }
                }
                Err(status) => log_event.log_error(status),
            },
            diff => match update_steam_data(firestore, &mut game_entry, igdb_game).await {
                Ok(()) => log_event.log_updated(diff),
                Err(status) => log_event.log_error(status),
            },
        },
        Err(Status::NotFound(_)) => {
            handle_add_game(igdb_game, firestore, resolver, game_filter, log_event).await
        }
        Err(status) => log_event.log_error(status),
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

async fn update_steam_data(
    firestore: Arc<FirestoreApi>,
    game_entry: &mut GameEntry,
    igdb_game: IgdbGame,
) -> Result<(), Status> {
    game_entry.update(igdb_game);

    // Spawn a task to retrieve steam data.
    let steam_handle =
        match firestore::external_games::get_steam_id(&firestore, game_entry.id).await {
            Ok(steam_appid) => Some(tokio::spawn(
                async move {
                    let steam = SteamDataApi::new();
                    steam.retrieve_steam_data(&steam_appid).await
                }
                .instrument(trace_span!("spawn_steam_request")),
            )),
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

#[instrument(level = "trace", skip(external_game, firestore))]
pub async fn external_games_webhook(
    external_game: IgdbExternalGame,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    if !(external_game.is_supported_store()) {
        return Ok(StatusCode::OK);
    }

    let mut external_game = ExternalGame::from(external_game);
    match external_game.store_name.as_str() {
        "gog" => {
            if let Some(url) = &external_game.store_url {
                match GogScrape::scrape(url).await {
                    Ok(gog_data) => external_game.gog_data = Some(gog_data),
                    Err(status) => warn!("GOG scraping failed: {status}"),
                }
            }
        }
        _ => {}
    }

    let result = firestore::external_games::write(&firestore, &external_game).await;
    let event = ExternalGameEvent::new(external_game);

    match result {
        Ok(()) => event.log(),
        Err(status) => event.log_error(status),
    }

    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(keyword, firestore))]
pub async fn keywords_webhook(
    keyword: Keyword,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    let result = firestore::keywords::write(&firestore, &keyword).await;
    let event = KeywordsEvent::new(keyword);

    match result {
        Ok(()) => event.log(),
        Err(status) => event.log_error(status),
    }

    Ok(StatusCode::OK)
}

const ADD_GAME_HANDLER: &str = "post_add_game";
const UPDATE_GAME_HANDLER: &str = "post_update_game";
