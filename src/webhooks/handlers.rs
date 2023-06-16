use crate::{
    api::{FirestoreApi, IgdbApi, IgdbGame},
    documents::{GameCategory, GameStatus},
    library::firestore,
    Status,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
use tracing::{error, info, instrument, log::warn};
use warp::http::StatusCode;

#[instrument(level = "trace", skip(firestore, igdb))]
pub async fn post_add_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    if !igdb_game.is_pc_game() || !igdb_game.has_hype() {
        return Ok(StatusCode::OK);
    }

    let game_id = igdb_game.id;
    let game_entry = match igdb.resolve(Arc::clone(&firestore), igdb_game).await {
        Ok(game_entry) => game_entry,
        Err(e) => {
            error!(
                labels.log_type = "webhook_logs",
                labels.handler = "post_add_game",
                labels.counter = "resolve_fail",
                game_added.id = game_id,
                game_added.error = e.to_string(),
                "failed to resolve {game_id}"
            );
            return Ok(StatusCode::OK);
        }
    };

    info!(
        labels.log_type = "webhook_logs",
        labels.handler = "post_add_game",
        labels.counter = "add_game",
        game_added.id = game_entry.id,
        "added '{}'",
        game_entry.name,
    );

    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(firestore, igdb))]
pub async fn post_update_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    if !igdb_game.is_pc_game() || !igdb_game.has_hype() {
        return Ok(StatusCode::OK);
    }

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

                    let mut firestore = firestore.lock().unwrap();
                    firestore.validate();
                    if let Err(e) = firestore::games::write(&firestore, &game_entry) {
                        error!(
                            labels.log_type = "webhook_logs",
                            labels.handler = "post_update_game",
                            labels.counter = "firestore_read_fail",
                            webhook.game_id = game_entry.igdb_game.id,
                            webhook.error = e.to_string(),
                            "failed to read '{}' ({})",
                            game_entry.igdb_game.name,
                            game_entry.igdb_game.id,
                        );
                    }
                }
                true => {
                    if let Err(e) = igdb.resolve(firestore, igdb_game).await {
                        error!(
                            labels.log_type = "webhook_logs",
                            labels.handler = "post_update_game",
                            labels.counter = "resolve_fail",
                            webhook.game_id = game_entry.id,
                            webhook.error = e.to_string(),
                            "failed to resolve '{}' ({})",
                            game_entry.name,
                            game_entry.id,
                        );
                    }
                }
            }

            info!(
                labels.log_type = "webhook_logs",
                labels.handler = "post_update_game",
                labels.counter = "update_game",
                webhook.game_id = game_entry.id,
                webhook.game_diff = diff.to_string(),
                "updated '{}'",
                game_entry.name,
            );
        }
        Err(Status::NotFound(_)) => {
            let game_id = igdb_game.id;
            let game_entry = match igdb.resolve(Arc::clone(&firestore), igdb_game).await {
                Ok(game_entry) => game_entry,
                Err(e) => {
                    error!(
                        labels.log_type = "webhook_logs",
                        labels.handler = "post_update_game",
                        labels.counter = "resolve_fail",
                        webhook.game_id = game_id,
                        webhook.error = e.to_string(),
                        "failed to resolve {game_id}"
                    );
                    return Ok(StatusCode::OK);
                }
            };

            info!(
                labels.log_type = "webhook_logs",
                labels.handler = "post_update_game",
                labels.counter = "add_game",
                webhook.game_id = game_entry.id,
                "added '{}'",
                game_entry.name,
            );
        }
        Err(e) => {
            error!(
                labels.log_type = "webhook_logs",
                labels.handler = "post_update_game",
                labels.counter = "firestore_read_fail",
                webhook.game_id = igdb_game.id,
                webhook.error = e.to_string(),
                "failed to read '{}' ({})",
                igdb_game.name,
                igdb_game.id,
            );
        }
    }

    Ok(StatusCode::OK)
}
