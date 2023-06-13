use crate::api::{FirestoreApi, IgdbApi, IgdbGame};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
use tracing::{info, instrument};
use warp::http::StatusCode;

#[instrument(level = "trace", skip(_firestore, _igdb))]
pub async fn post_game_added_webhook(
    igdb_game: IgdbGame,
    _firestore: Arc<Mutex<FirestoreApi>>,
    _igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    info!(
        labels.log_type = "webhook_logs",
        labels.handler = "game_added",
        "Game added: {:?}",
        igdb_game
    );
    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(_firestore, _igdb))]
pub async fn post_game_updated_webhook(
    igdb_game: IgdbGame,
    _firestore: Arc<Mutex<FirestoreApi>>,
    _igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    info!(
        labels.log_type = "webhook_logs",
        labels.handler = "game_updated",
        "Game updated: {:?}",
        igdb_game
    );
    Ok(StatusCode::OK)
}
