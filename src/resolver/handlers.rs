use crate::{api::FirestoreApi, documents::IgdbGame, Status};

use std::{convert::Infallible, sync::Arc};
use tracing::{error, instrument};
use warp::http::StatusCode;

use super::{
    igdb::{IgdbApi, IgdbSearch},
    models::SearchRequest,
};

#[instrument(level = "trace", skip(firestore, igdb,))]
pub async fn post_retrieve(
    id: u64,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    match igdb.get(id).await {
        Ok(igdb_game) => match igdb.resolve_only(firestore, igdb_game).await {
            Ok(game_entry) => Ok(Box::new(warp::reply::json(&game_entry))),
            Err(Status::NotFound(_)) => Ok(Box::new(StatusCode::NOT_FOUND)),
            Err(status) => {
                error!("post_digest: {status}");
                Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
            }
        },
        Err(Status::NotFound(_)) => Ok(Box::new(StatusCode::NOT_FOUND)),
        Err(status) => {
            error!("post_digest: {status}");
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[instrument(level = "trace", skip(igdb_game, firestore, igdb,))]
pub async fn post_resolve(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    match igdb.resolve_only(firestore, igdb_game).await {
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
) -> Result<Box<dyn warp::Reply>, Infallible> {
    match igdb.get(id).await {
        Ok(igdb_game) => match igdb.resolve_digest(&firestore, igdb_game).await {
            Ok(digest) => Ok(Box::new(warp::reply::json(&digest))),
            Err(Status::NotFound(_)) => Ok(Box::new(StatusCode::NOT_FOUND)),
            Err(status) => {
                error!("post_digest: {status}");
                Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
            }
        },
        Err(Status::NotFound(_)) => Ok(Box::new(StatusCode::NOT_FOUND)),
        Err(status) => {
            error!("post_digest: {status}");
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[instrument(level = "trace", skip(firestore, igdb))]
pub async fn post_search(
    request: SearchRequest,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let igdb_search = IgdbSearch::new(igdb);
    match igdb_search
        .search_by_title(&firestore, &request.title)
        .await
    {
        Ok(candidates) => Ok(Box::new(warp::reply::json(&candidates))),
        Err(Status::NotFound(_)) => Ok(Box::new(StatusCode::NOT_FOUND)),
        Err(status) => {
            error!("post_search: {status}");
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}
