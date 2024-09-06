use crate::{
    api::{
        FirestoreApi, GogScrape, IgdbApi, IgdbExternalGame, IgdbGame, MetacriticApi, SteamDataApi,
        SteamScrape,
    },
    documents::{ExternalGame, GameEntry, Keyword},
    library::firestore,
    Status,
};
use chrono::Utc;
use std::{convert::Infallible, sync::Arc};
use tracing::{instrument, trace_span, warn, Instrument};
use warp::http::StatusCode;

use super::{
    event_logs::{AddGameEvent, ExternalGameEvent, KeywordsEvent, UpdateGameEvent},
    filtering::GameFilter,
    prefiltering::IgdbPrefilter,
};

#[instrument(level = "trace", skip(igdb_game, firestore, igdb, game_filter))]
pub async fn add_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
    game_filter: Arc<GameFilter>,
) -> Result<impl warp::Reply, Infallible> {
    let event = AddGameEvent::new(igdb_game.id, igdb_game.name.clone());

    if !IgdbPrefilter::filter(&igdb_game) {
        event.log_prefilter_reject(IgdbPrefilter::explain(&igdb_game));
        return Ok(StatusCode::OK);
    }

    match igdb
        .resolve_only(Arc::clone(&firestore), igdb_game, &game_filter)
        .await
    {
        Ok((mut game_entry, rejection)) => {
            if let Some(rejection) = rejection {
                event.log_reject(rejection);
            } else if let Err(status) = firestore::games::write(&firestore, &mut game_entry).await {
                event.log_error(status);
            } else {
                event.log()
            }
        }
        Err(status) => event.log_error(status),
    }

    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(igdb_game, firestore, igdb, game_filter))]
pub async fn update_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
    game_filter: Arc<GameFilter>,
) -> Result<impl warp::Reply, Infallible> {
    let event = UpdateGameEvent::new(igdb_game.id, igdb_game.name.clone());

    if !IgdbPrefilter::filter(&igdb_game) {
        event.log_prefilter_reject(IgdbPrefilter::explain(&igdb_game));
        return Ok(StatusCode::OK);
    }

    let game_entry = firestore::games::read(&firestore, igdb_game.id).await;

    match game_entry {
        Ok(mut game_entry) => match game_entry.igdb_game.diff(&igdb_game) {
            diff if diff.empty() => {
                if needs_update(&game_entry) {
                    match update_steam_data(firestore, &mut game_entry, igdb_game).await {
                        Ok(()) => event.log(Some(diff)),
                        Err(status) => event.log_error(status),
                    }
                } else {
                    event.log(None)
                }
            }
            diff if diff.needs_resolve() => match igdb.resolve(firestore, igdb_game).await {
                Ok(_) => event.log(Some(diff)),
                Err(status) => event.log_error(status),
            },
            diff => match update_steam_data(firestore, &mut game_entry, igdb_game).await {
                Ok(()) => event.log(Some(diff)),
                Err(status) => event.log_error(status),
            },
        },
        Err(Status::NotFound(_)) => {
            match igdb
                .resolve_only(Arc::clone(&firestore), igdb_game, &game_filter)
                .await
            {
                Ok((mut game_entry, rejection)) => {
                    if let Some(rejection) = rejection {
                        event.log_reject(rejection);
                    } else if let Err(status) =
                        firestore::games::write(&firestore, &mut game_entry).await
                    {
                        event.log_error(status);
                    } else {
                        event.log_added()
                    }
                }
                Err(status) => event.log_error(status),
            }
        }
        Err(status) => event.log_error(status),
    }

    Ok(StatusCode::OK)
}

fn needs_update(game_entry: &GameEntry) -> bool {
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
