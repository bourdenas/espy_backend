use crate::{
    api::{FirestoreApi, IgdbApi, IgdbGame},
    Status,
};

use std::{convert::Infallible, sync::Arc};
use tracing::{error, instrument};
use warp::http::StatusCode;

#[instrument(level = "trace", skip(firestore, igdb,))]
pub async fn post_retrieve(
    id: u64,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::OK)
}

#[instrument(level = "trace", skip(igdb_game, firestore, igdb,))]
pub async fn post_resolve(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    match igdb.resolve_only(Arc::clone(&firestore), igdb_game).await {
        Ok(game_entry) => Ok(Box::new(warp::reply::json(&game_entry))),
        Err(Status::NotFound(_)) => Ok(Box::new(StatusCode::NOT_FOUND)),
        Err(status) => {
            error!("post_resolve: {status}");
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[instrument(level = "trace", skip(firestore, igdb))]
pub async fn post_digest(
    id: u64,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::OK)
}
