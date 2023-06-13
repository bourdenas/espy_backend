use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
use tracing::warn;
use warp::{self, Filter};

use crate::api::{FirestoreApi, IgdbApi, IgdbGame};

use super::handlers;

/// Returns a Filter with all available routes.
pub fn routes(
    igdb: Arc<IgdbApi>,
    firestore: Arc<Mutex<FirestoreApi>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    post_game_added_webhook(Arc::clone(&firestore), Arc::clone(&igdb))
        .or(post_game_updated_webhook(firestore, igdb))
        .or_else(|e| async {
            warn! {"Rejected route: {:?}", e};
            Err(e)
        })
}

/// POST /game_added
fn post_game_added_webhook(
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("game_added")
        .and(warp::post())
        .and(json_body::<IgdbGame>())
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::post_game_added_webhook)
}

/// POST /game_updated
fn post_game_updated_webhook(
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("game_updated")
        .and(warp::post())
        .and(json_body::<IgdbGame>())
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::post_game_updated_webhook)
}

fn json_body<T: serde::de::DeserializeOwned + Send>(
) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(16 * 1024).and(warp::body::json())
}

pub fn with_igdb(
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (Arc<IgdbApi>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&igdb))
}

pub fn with_firestore(
    firestore: Arc<Mutex<FirestoreApi>>,
) -> impl Filter<Extract = (Arc<Mutex<FirestoreApi>>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&firestore))
}
