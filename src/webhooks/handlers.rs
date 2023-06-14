use crate::{
    api::{FirestoreApi, IgdbApi, IgdbGame},
    library::firestore,
    Status,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
use tracing::{error, info, instrument};
use warp::http::StatusCode;

#[instrument(level = "trace", skip(firestore, igdb))]
pub async fn post_add_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    info!(
        labels.log_type = "webhook_logs",
        labels.handler = "post_add_game",
        game_added.id = igdb_game.id,
    );

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
            );
            return Ok(StatusCode::OK);
        }
    };

    info!(
        labels.log_type = "webhook_logs",
        labels.handler = "post_add_game",
        labels.counter = "add_game",
        game_added.id = game_entry.id,
    );

    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(firestore, igdb))]
pub async fn post_update_game_webhook(
    igdb_game: IgdbGame,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    info!(
        labels.log_type = "webhook_logs",
        labels.counter = "post_game_updated",
        igdb.game_id = igdb_game.id,
    );

    if !igdb_game.is_pc_game() || !igdb_game.has_hype() {
        return Ok(StatusCode::OK);
    }

    let game_entry = {
        let mut firestore = firestore.lock().unwrap();
        firestore.validate();
        firestore::games::read(&firestore, igdb_game.id)
    };
    match game_entry {
        Ok(game_entry) => {
            // let diff = game_entry.igdb_game.diff(&igdb_game);
            info!(
                labels.log_type = "webhook_logs",
                labels.handler = "post_update_game",
                labels.counter = "update_game",
                game_added.id = game_entry.id,
            );
        }
        Err(Status::NotFound(_)) => {
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
                    );
                    return Ok(StatusCode::OK);
                }
            };

            info!(
                labels.log_type = "webhook_logs",
                labels.handler = "post_update_game",
                labels.counter = "add_game",
                game_added.id = game_entry.id,
            );
        }
        Err(e) => {
            error!(
                labels.log_type = "webhook_logs",
                labels.handler = "post_add_game",
                labels.counter = "resolve_fail",
                game_added.id = igdb_game.id,
                game_added.error = e.to_string(),
            );
        }
    }

    Ok(StatusCode::OK)
}
