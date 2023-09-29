use crate::{
    api::{FirestoreApi, IgdbApi, IgdbExternalGame, IgdbGame},
    documents::{ExternalGame, GameCategory, GameStatus, Genre, Keyword},
    games::SteamDataApi,
    library::firestore,
    logging::{AddGameEvent, UpdateGameEvent},
    Status,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
use tracing::{error, info, instrument, log::warn};
use warp::http::StatusCode;

#[instrument(level = "trace", skip(igdb_game, firestore, igdb))]
pub async fn add_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    if !igdb_game.is_pc_game() || !igdb_game.has_hype() {
        return Ok(StatusCode::OK);
    }

    let event = AddGameEvent::new(igdb_game.id, igdb_game.name.clone());
    match igdb.resolve(Arc::clone(&firestore), igdb_game).await {
        Ok(_) => event.log(),
        Err(status) => event.log_error(status),
    }

    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(igdb_game, firestore, igdb))]
pub async fn update_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    if !igdb_game.is_pc_game() || !igdb_game.has_hype() {
        return Ok(StatusCode::OK);
    }

    let event = UpdateGameEvent::new(igdb_game.id, igdb_game.name.clone());

    let game_entry = {
        let mut firestore = firestore.lock().unwrap();
        firestore.validate();
        firestore::games::read(&firestore, igdb_game.id)
    };
    match game_entry {
        Ok(mut game_entry) => {
            let diff = game_entry.igdb_game.diff(&igdb_game);
            let reverse_diff = igdb_game.diff(&game_entry.igdb_game);
            if diff.empty() && reverse_diff.empty() {
                event.log(None);
                return Ok(StatusCode::OK);
            }

            if reverse_diff.needs_resolve() {
                warn!("remove game from companies/collections if necessary");
            }

            match diff.needs_resolve() {
                false => {
                    game_entry.igdb_game = igdb_game;
                    if let Some(name) = &diff.name {
                        game_entry.name = name.clone();
                    }
                    if let Some(category) = diff.category {
                        game_entry.category = GameCategory::from(category);
                    }
                    if let Some(status) = diff.status {
                        game_entry.status = GameStatus::from(status);
                    }

                    let steam = SteamDataApi::new();
                    if let Err(e) = steam.retrieve_steam_data(&mut game_entry).await {
                        warn!("Failed to retrieve SteamData for '{}' {e}", game_entry.name);
                    }

                    let mut firestore = firestore.lock().unwrap();
                    firestore.validate();
                    if let Err(status) = firestore::games::write(&firestore, &game_entry) {
                        event.log_error(status);
                        return Ok(StatusCode::OK);
                    }
                }
                true => {
                    if let Err(status) = igdb.resolve(firestore, igdb_game).await {
                        event.log_error(status);
                        return Ok(StatusCode::OK);
                    }
                }
            }

            event.log(Some(diff));
        }
        Err(Status::NotFound(_)) => match igdb.resolve(Arc::clone(&firestore), igdb_game).await {
            Ok(_) => event.log_added(),
            Err(status) => event.log_error(status),
        },
        Err(status) => event.log_error(status),
    }

    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(external_game, firestore))]
pub async fn external_games_webhook(
    external_game: IgdbExternalGame,
    firestore: Arc<Mutex<FirestoreApi>>,
) -> Result<impl warp::Reply, Infallible> {
    if !(external_game.is_steam() || external_game.is_gog()) {
        return Ok(StatusCode::OK);
    }

    let external_game = ExternalGame::from(external_game);

    let mut firestore = firestore.lock().unwrap();
    firestore.validate();
    let result = firestore::external_games::write(&firestore, &external_game);

    match result {
        Ok(()) => info!(
            labels.log_type = "webhook_logs",
            labels.handler = "post_external_game",
            labels.counter = "update_external_game",
            external_game.store = external_game.store_name,
            external_game.store_id = external_game.store_id,
            "external game updated",
        ),
        Err(e) => error!(
            labels.log_type = "webhook_logs",
            labels.handler = "post_external_game",
            labels.counter = "update_external_game",
            external_game.store = external_game.store_name,
            external_game.store_id = external_game.store_id,
            external_game.error = e.to_string(),
            "failed to store {:?}",
            external_game,
        ),
    }

    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(genre, firestore))]
pub async fn genres_webhook(
    genre: Genre,
    firestore: Arc<Mutex<FirestoreApi>>,
) -> Result<impl warp::Reply, Infallible> {
    let mut firestore = firestore.lock().unwrap();
    firestore.validate();
    let result = firestore::genres::write(&firestore, &genre);

    match result {
        Ok(()) => info!(
            labels.log_type = "webhook_logs",
            labels.handler = "post_genres",
            labels.counter = "update_genre",
            genre.name = genre.name,
            "genre updated",
        ),
        Err(e) => error!(
            labels.log_type = "webhook_logs",
            labels.handler = "post_genres",
            labels.counter = "update_genre",
            genre.name = genre.name,
            genre.error = e.to_string(),
            "failed to strore {:?}",
            genre,
        ),
    }

    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(keyword, firestore))]
pub async fn keywords_webhook(
    keyword: Keyword,
    firestore: Arc<Mutex<FirestoreApi>>,
) -> Result<impl warp::Reply, Infallible> {
    let mut firestore = firestore.lock().unwrap();
    firestore.validate();
    let result = firestore::keywords::write(&firestore, &keyword);

    match result {
        Ok(()) => info!(
            labels.log_type = "webhook_logs",
            labels.handler = "post_keywords",
            labels.counter = "update_keyword",
            keyword.name = keyword.name,
            "keyword updated",
        ),
        Err(e) => error!(
            labels.log_type = "webhook_logs",
            labels.handler = "post_keywords",
            labels.counter = "update_keyword",
            keyword.name = keyword.name,
            keyword.error = e.to_string(),
            "failed to strore {:?}",
            keyword,
        ),
    }

    Ok(StatusCode::OK)
}
