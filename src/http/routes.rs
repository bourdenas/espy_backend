use crate::{
    api::{FirestoreApi, IgdbApi},
    util,
};
use std::sync::Arc;
use tracing::warn;
use warp::{self, Filter};

use super::{handlers, models, resources::*};

/// Returns a Filter with all available routes.
pub fn routes(
    keys: Arc<util::keys::Keys>,
    igdb: Arc<IgdbApi>,
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    home()
        .or(post_search(Arc::clone(&igdb)))
        .or(post_resolve(Arc::clone(&firestore), Arc::clone(&igdb)))
        .or(post_delete(Arc::clone(&firestore)))
        .or(post_match(Arc::clone(&firestore), Arc::clone(&igdb)))
        .or(post_update(Arc::clone(&firestore)))
        .or(post_wishlist(Arc::clone(&firestore)))
        .or(post_unlink(Arc::clone(&firestore)))
        .or(post_sync(keys, Arc::clone(&firestore), Arc::clone(&igdb)))
        .or(get_images())
        .or_else(|e| async {
            warn! {"Rejected route: {:?}", e};
            Err(e)
        })
}

/// GET /
fn home() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!().and(warp::get()).and_then(handlers::welcome)
}

/// POST /search
fn post_search(
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("search")
        .and(warp::post())
        .and(json_body::<models::Search>())
        .and(with_igdb(igdb))
        .and_then(handlers::post_search)
}

/// POST /resolve
fn post_resolve(
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("resolve")
        .and(warp::post())
        .and(json_body::<models::Resolve>())
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::post_resolve)
}

/// POST /delete
fn post_delete(
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("delete")
        .and(warp::post())
        .and(json_body::<models::Resolve>())
        .and(with_firestore(firestore))
        .and_then(handlers::post_delete)
}

/// POST /library/{user_id}/match
fn post_match(
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("library" / String / "match")
        .and(warp::post())
        .and(json_body::<models::MatchOp>())
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::post_match)
}

/// POST /library/{user_id}/update
fn post_update(
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("library" / String / "update")
        .and(warp::post())
        .and(json_body::<models::UpdateOp>())
        .and(with_firestore(firestore))
        .and_then(handlers::post_update)
}

/// POST /library/{user_id}/wishlist
fn post_wishlist(
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("library" / String / "wishlist")
        .and(warp::post())
        .and(json_body::<models::WishlistOp>())
        .and(with_firestore(firestore))
        .and_then(handlers::post_wishlist)
}

/// POST /library/{user_id}/unlink
fn post_unlink(
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("library" / String / "unlink")
        .and(warp::post())
        .and(json_body::<models::Unlink>())
        .and(with_firestore(firestore))
        .and_then(handlers::post_unlink)
}

/// POST /library/{user_id}/sync
fn post_sync(
    keys: Arc<util::keys::Keys>,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("library" / String / "sync")
        .and(warp::post())
        .and(with_keys(keys))
        .and(with_firestore(firestore))
        .and(with_igdb(igdb))
        .and_then(handlers::post_sync)
}

/// GET /images/{resolution}/{image_id}
fn get_images() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("images" / String)
        .and(warp::get())
        .and_then(handlers::get_images)
}

fn json_body<T: serde::de::DeserializeOwned + Send>(
) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(16 * 1024).and(warp::body::json())
}
