use crate::{api::FirestoreApi, logging::LogResolverRequest, Status};

use std::{convert::Infallible, sync::Arc};
use tracing::instrument;
use warp::http::StatusCode;

use super::{
    igdb::{filtering::GameFilter, IgdbApi, IgdbSearch},
    models::{ResolveRequest, ResolveResponse, SearchRequest},
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
    skip(request, firestore, filter, connection)
)]
pub async fn post_resolve(
    request: ResolveRequest,
    firestore: Arc<FirestoreApi>,
    filter: Arc<GameFilter>,
    connection: Arc<IgdbConnection>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let id = request.igdb_game.id;
    let igdb = IgdbApi::new(connection);

    let response = match request.filter {
        true => {
            igdb.resolve_filter(firestore, filter, request.igdb_game)
                .await
        }
        false => match igdb.resolve(firestore, request.igdb_game).await {
            Ok(game_entry) => Ok(ResolveResponse::Success(game_entry)),
            Err(status) => Err(status),
        },
    };

    match response {
        Ok(response) => {
            match &response {
                ResolveResponse::Success(game_entry) => {
                    LogResolverRequest::resolve(id, Some(game_entry.name.clone()), Status::Ok)
                }
                ResolveResponse::Reject(_reason) => {
                    LogResolverRequest::resolve(id, None, Status::Ok)
                }
            }
            Ok(Box::new(warp::reply::json(&response)))
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
