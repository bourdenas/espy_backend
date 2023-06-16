use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
use tracing::warn;
use warp::{self, Filter};

use crate::{
    api::{FirestoreApi, IgdbApi, IgdbExternalGame, IgdbGame},
    documents::{Genre, Keyword},
};

use super::handlers;

/// Returns a Filter with all available routes.
pub fn routes(
    igdb: Arc<IgdbApi>,
    firestore: Arc<Mutex<FirestoreApi>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    post_add_game(Arc::clone(&firestore), Arc::clone(&igdb))
        .or(post_update_game(Arc::clone(&firestore), Arc::clone(&igdb)))
        .or(post_external_game(Arc::clone(&firestore)))
        .or(post_genres(Arc::clone(&firestore)))
        .or(post_keywords(Arc::clone(&firestore)))
        .or_else(|e| async {
            warn! {"Rejected route: {:?}", e};
            Err(e)
        })
}

/// POST /add_game
fn post_add_game(
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("add_game")
        .and(warp::post())
        .and(json_body::<IgdbGame>())
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::add_game_webhook)
}

/// POST /update_game
fn post_update_game(
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("update_game")
        .and(warp::post())
        .and(json_body::<IgdbGame>())
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::update_game_webhook)
}

/// POST /external_games
fn post_external_game(
    firestore: Arc<Mutex<FirestoreApi>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("external_games")
        .and(warp::post())
        .and(json_body::<IgdbExternalGame>())
        .and(with_firestore(firestore))
        .and_then(handlers::external_games_webhook)
}

/// POST /genres
fn post_genres(
    firestore: Arc<Mutex<FirestoreApi>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("genres")
        .and(warp::post())
        .and(json_body::<Genre>())
        .and(with_firestore(firestore))
        .and_then(handlers::genres_webhook)
}

/// POST /keywords
fn post_keywords(
    firestore: Arc<Mutex<FirestoreApi>>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("keywords")
        .and(warp::post())
        .and(json_body::<Keyword>())
        .and(with_firestore(firestore))
        .and_then(handlers::keywords_webhook)
}

fn json_body<T: serde::de::DeserializeOwned + Send>(
) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(32 * 1024).and(warp::body::json())
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
