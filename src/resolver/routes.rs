use std::{convert::Infallible, sync::Arc};
use tracing::warn;
use warp::{self, Filter};

use crate::{api::FirestoreApi, documents::IgdbGame};

use super::{handlers, igdb::IgdbApi, models::SearchRequest};

/// Returns a Filter with all available routes.
pub fn routes(
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    post_retrieve(Arc::clone(&firestore), Arc::clone(&igdb))
        .or(post_resolve(Arc::clone(&firestore), Arc::clone(&igdb)))
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
    igdb: Arc<IgdbApi>,
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
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("resolve")
        .and(warp::post())
        .and(json_body::<IgdbGame>())
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::post_resolve)
}

/// POST /digest
fn post_digest(
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
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
    igdb: Arc<IgdbApi>,
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

pub fn with_firestore(
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (Arc<FirestoreApi>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&firestore))
}

pub fn with_igdb(
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (Arc<IgdbApi>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&igdb))
}
