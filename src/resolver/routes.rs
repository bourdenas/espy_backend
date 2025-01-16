use std::{convert::Infallible, sync::Arc};
use tracing::warn;
use warp::{self, Filter};

use crate::api::FirestoreApi;

use super::{
    handlers,
    igdb::filtering::GameFilter,
    models::{ResolveRequest, SearchRequest},
    IgdbConnection,
};

/// Returns a Filter with all available routes.
pub fn routes(
    firestore: Arc<FirestoreApi>,
    filter: Arc<GameFilter>,
    igdb: Arc<IgdbConnection>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    post_retrieve(Arc::clone(&firestore), Arc::clone(&igdb))
        .or(post_resolve(
            Arc::clone(&firestore),
            Arc::clone(&filter),
            Arc::clone(&igdb),
        ))
        .or(post_digest(Arc::clone(&firestore), Arc::clone(&igdb)))
        .or(post_search(Arc::clone(&firestore), Arc::clone(&igdb)))
        .or_else(|e| async {
            warn! {"Rejected route: {:?}", e};
            Err(e)
        })
}

/// POST /retrieve
fn post_retrieve(
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbConnection>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("retrieve")
        .and(warp::post())
        .and(json_body::<u64>())
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::post_retrieve)
}

/// POST /resolve
fn post_resolve(
    firestore: Arc<FirestoreApi>,
    filter: Arc<GameFilter>,
    igdb: Arc<IgdbConnection>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("resolve")
        .and(warp::post())
        .and(json_body::<ResolveRequest>())
        .and(with_firestore(firestore))
        .and(with_filter(filter))
        .and(with_igdb(igdb))
        .and_then(handlers::post_resolve)
}

/// POST /digest
fn post_digest(
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbConnection>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("digest")
        .and(warp::post())
        .and(json_body::<u64>())
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::post_digest)
}

/// POST /search
fn post_search(
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbConnection>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("search")
        .and(warp::post())
        .and(json_body::<SearchRequest>())
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::post_search)
}

fn json_body<T: serde::de::DeserializeOwned + Send>(
) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(32 * 1024).and(warp::body::json())
}

fn with_firestore(
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (Arc<FirestoreApi>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&firestore))
}

fn with_filter(
    classifier: Arc<GameFilter>,
) -> impl Filter<Extract = (Arc<GameFilter>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&classifier))
}

fn with_igdb(
    igdb: Arc<IgdbConnection>,
) -> impl Filter<Extract = (Arc<IgdbConnection>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&igdb))
}
