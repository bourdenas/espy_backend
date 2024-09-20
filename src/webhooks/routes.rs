use std::{convert::Infallible, sync::Arc};
use tracing::warn;
use warp::{self, Filter};

use crate::{
    api::FirestoreApi,
    documents::{IgdbExternalGame, IgdbGame, Keyword},
    resolver::ResolveApi,
};

use super::{filtering::GameFilter, handlers};

/// Returns a Filter with all available routes.
pub fn routes(
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    classifier: Arc<GameFilter>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    post_add_game(
        Arc::clone(&firestore),
        Arc::clone(&resolver),
        Arc::clone(&classifier),
    )
    .or(post_update_game(
        Arc::clone(&firestore),
        Arc::clone(&resolver),
        Arc::clone(&classifier),
    ))
    .or(post_external_game(Arc::clone(&firestore)))
    .or(post_keywords(Arc::clone(&firestore)))
    .or_else(|e| async {
        warn! {"Rejected route: {:?}", e};
        Err(e)
    })
}

/// POST /add_game
fn post_add_game(
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    classifier: Arc<GameFilter>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("add_game")
        .and(warp::post())
        .and(json_body::<IgdbGame>())
        .and(with_firestore(firestore))
        .and(with_resolver(resolver))
        .and(with_classifier(classifier))
        .and_then(handlers::add_game_webhook)
}

/// POST /update_game
fn post_update_game(
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    classifier: Arc<GameFilter>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("update_game")
        .and(warp::post())
        .and(json_body::<IgdbGame>())
        .and(with_firestore(firestore))
        .and(with_resolver(resolver))
        .and(with_classifier(classifier))
        .and_then(handlers::update_game_webhook)
}

/// POST /external_games
fn post_external_game(
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("external_games")
        .and(warp::post())
        .and(json_body::<IgdbExternalGame>())
        .and(with_firestore(firestore))
        .and_then(handlers::external_games_webhook)
}

/// POST /keywords
fn post_keywords(
    firestore: Arc<FirestoreApi>,
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

pub fn with_firestore(
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (Arc<FirestoreApi>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&firestore))
}

pub fn with_resolver(
    resolver: Arc<ResolveApi>,
) -> impl Filter<Extract = (Arc<ResolveApi>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&resolver))
}

pub fn with_classifier(
    classifier: Arc<GameFilter>,
) -> impl Filter<Extract = (Arc<GameFilter>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&classifier))
}
