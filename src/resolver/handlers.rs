use crate::{api::FirestoreApi, documents::IgdbGame, logging::LogResolverRequest, Status};

use std::{convert::Infallible, sync::Arc};
use tracing::instrument;
use warp::http::StatusCode;

use super::{
    igdb::{IgdbApi, IgdbSearch},
    models::SearchRequest,
    IgdbConnection,
};

#[instrument(name = "retrieve", level = "info", skip(firestore, connection))]
pub async fn post_retrieve(
    id: u64,
    firestore: Arc<FirestoreApi>,
    connection: Arc<IgdbConnection>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let igdb = IgdbApi::new(connection);
    match igdb.get(id).await {
        Ok(igdb_game) => match igdb.resolve(firestore, igdb_game).await {
            Ok(game_entry) => {
                LogResolverRequest::retrieve(id, Some(game_entry.name.clone()), Status::Ok);
                Ok(Box::new(warp::reply::json(&game_entry)))
            }
            Err(Status::NotFound(msg)) => {
                LogResolverRequest::retrieve(id, None, Status::not_found(msg));
                Ok(Box::new(StatusCode::NOT_FOUND))
            }
            Err(status) => {
                LogResolverRequest::retrieve(id, None, status);
                Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
            }
        },
        Err(Status::NotFound(msg)) => {
            LogResolverRequest::retrieve(id, None, Status::not_found(msg));
            Ok(Box::new(StatusCode::NOT_FOUND))
        }
        Err(status) => {
            LogResolverRequest::retrieve(id, None, status);
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[instrument(
    name = "resolve",
    level = "info",
    skip(igdb_game, firestore, connection)
)]
pub async fn post_resolve(
    igdb_game: IgdbGame,
    firestore: Arc<FirestoreApi>,
    connection: Arc<IgdbConnection>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let id = igdb_game.id;
    let igdb = IgdbApi::new(connection);
    match igdb.resolve(firestore, igdb_game).await {
        Ok(game_entry) => {
            LogResolverRequest::resolve(id, Some(game_entry.name.clone()), Status::Ok);
            Ok(Box::new(warp::reply::json(&game_entry)))
        }
        Err(Status::NotFound(msg)) => {
            LogResolverRequest::resolve(id, None, Status::not_found(msg));
            Ok(Box::new(StatusCode::NOT_FOUND))
        }
        Err(status) => {
            LogResolverRequest::resolve(id, None, status);
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[instrument(name = "digest", level = "info", skip(firestore, connection))]
pub async fn post_digest(
    id: u64,
    firestore: Arc<FirestoreApi>,
    connection: Arc<IgdbConnection>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let igdb = IgdbApi::new(connection);
    match igdb.get(id).await {
        Ok(igdb_game) => match igdb.resolve_digest(&firestore, igdb_game).await {
            Ok(digest) => {
                LogResolverRequest::digest(id, Some(digest.name.clone()), Status::Ok);
                Ok(Box::new(warp::reply::json(&digest)))
            }
            Err(Status::NotFound(msg)) => {
                LogResolverRequest::digest(id, None, Status::not_found(msg));
                Ok(Box::new(StatusCode::NOT_FOUND))
            }
            Err(status) => {
                LogResolverRequest::digest(id, None, status);
                Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
            }
        },
        Err(Status::NotFound(msg)) => {
            LogResolverRequest::digest(id, None, Status::not_found(msg));
            Ok(Box::new(StatusCode::NOT_FOUND))
        }
        Err(status) => {
            LogResolverRequest::digest(id, None, status);
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[instrument(name = "search", level = "info", skip(firestore, connection))]
pub async fn post_search(
    request: SearchRequest,
    firestore: Arc<FirestoreApi>,
    connection: Arc<IgdbConnection>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let igdb_search = IgdbSearch::new(connection);
    match igdb_search
        .search_by_title(&firestore, &request.title)
        .await
    {
        Ok(candidates) => Ok(Box::new(warp::reply::json(&candidates))),
        Err(Status::NotFound(msg)) => {
            LogResolverRequest::search(request, &vec![], Status::not_found(msg));
            Ok(Box::new(StatusCode::NOT_FOUND))
        }
        Err(status) => {
            LogResolverRequest::search(request, &vec![], status);
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}
