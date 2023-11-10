use crate::{
    api::{FirestoreApi, IgdbApi, IgdbExternalGame, IgdbGame},
    documents::{ExternalGame, GameCategory, GameEntry, GameStatus, Genre, Keyword},
    games::SteamDataApi,
    library::firestore,
    Status,
};
use std::{
    convert::Infallible,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{instrument, warn};
use warp::http::StatusCode;

use super::event_logs::{
    AddGameEvent, ExternalGameEvent, GenresEvent, KeywordsEvent, UpdateGameEvent,
};

#[instrument(level = "trace", skip(igdb_game, firestore, igdb))]
pub async fn add_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    if !igdb_game.is_pc_game() || !igdb_game.is_main_category() {
        return Ok(StatusCode::OK);
    }

    let event = AddGameEvent::new(igdb_game.id, igdb_game.name.clone());
    match igdb.resolve(firestore, igdb_game).await {
        Ok(_) => event.log(),
        Err(status) => event.log_error(status),
    }

    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(igdb_game, firestore, igdb))]
pub async fn update_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    if !igdb_game.is_pc_game() || !igdb_game.is_main_category() {
        return Ok(StatusCode::OK);
    }

    let event = UpdateGameEvent::new(igdb_game.id, igdb_game.name.clone());
    let game_entry = firestore::games::read(&firestore, igdb_game.id).await;

    match game_entry {
        Ok(mut game_entry) => match game_entry.igdb_game.diff(&igdb_game) {
            diff if diff.empty() => {
                if game_entry.last_updated == 0
                    || SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .checked_sub(Duration::from_secs(game_entry.last_updated))
                        .unwrap()
                        > Duration::from_secs(15 * DAYS)
                {
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
        Err(Status::NotFound(_)) => match igdb.resolve(Arc::clone(&firestore), igdb_game).await {
            Ok(_) => event.log_added(),
            Err(status) => event.log_error(status),
        },
        Err(status) => event.log_error(status),
    }

    Ok(StatusCode::OK)
}

const DAYS: u64 = 24 * 60 * 60;

async fn update_steam_data(
    firestore: Arc<FirestoreApi>,
    game_entry: &mut GameEntry,
    igdb_game: IgdbGame,
) -> Result<(), Status> {
    game_entry.name = igdb_game.name.clone();
    game_entry.category = GameCategory::from(igdb_game.category);
    game_entry.status = GameStatus::from(igdb_game.status);
    game_entry.igdb_game = igdb_game;

    let steam = SteamDataApi::new();
    if let Err(e) = steam.retrieve_steam_data(game_entry).await {
        warn!("Failed to retrieve SteamData for '{}' {e}", game_entry.name);
    }

    firestore::games::write(&firestore, game_entry).await
}

#[instrument(level = "trace", skip(external_game, firestore))]
pub async fn external_games_webhook(
    external_game: IgdbExternalGame,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    if !(external_game.is_steam() || external_game.is_gog()) {
        return Ok(StatusCode::OK);
    }

    let external_game = ExternalGame::from(external_game);
    let result = firestore::external_games::write(&firestore, &external_game).await;
    let event = ExternalGameEvent::new(external_game);

    match result {
        Ok(()) => event.log(),
        Err(status) => event.log_error(status),
    }

    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(genre, firestore))]
pub async fn genres_webhook(
    genre: Genre,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    let result = firestore::genres::write(&firestore, &genre).await;
    let event = GenresEvent::new(genre);

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
