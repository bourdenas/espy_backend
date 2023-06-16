use crate::{
    api::{FirestoreApi, IgdbApi},
    http::models,
    library::{LibraryManager, User},
    util, Status,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tracing::{debug, error, info, instrument, warn};
use warp::http::StatusCode;

#[instrument(level = "trace")]
pub async fn welcome() -> Result<impl warp::Reply, Infallible> {
    info!(
        http_request.request_method = "GET",
        http_request.request_url = "/",
        labels.log_type = "query_logs",
        labels.handler = "welcome",
        "welcome"
    );
    Ok("welcome")
}

#[instrument(level = "trace", skip(igdb))]
pub async fn post_search(
    search: models::Search,
    igdb: Arc<IgdbApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let started = SystemTime::now();
    let response = match igdb
        .search_by_title_with_cover(&search.title, search.base_game_only)
        .await
    {
        Ok(candidates) => Ok(candidates),
        Err(e) => Err(Status::internal(format!(
            "search_by_title_with_cover(): {e}"
        ))),
    };
    let resp_time = SystemTime::now().duration_since(started).unwrap();

    match response {
        Ok(candidates) => {
            info!(
                http_request.request_method = "POST",
                http_request.request_url = "/search",
                labels.log_type = "query_logs",
                labels.handler = "search",
                search.title = search.title,
                search.latency = resp_time.as_millis(),
                search.results = candidates.len(),
                "search '{}'",
                search.title
            );
            Ok(Box::new(warp::reply::json(&candidates)))
        }
        Err(e) => {
            error!(
                http_request.request_method = "POST",
                http_request.request_url = "/search",
                labels.log_type = "query_logs",
                labels.handler = "search",
                search.title = search.title,
                search.latency = resp_time.as_millis(),
                search.error = e.to_string(),
                "search '{}'",
                search.title
            );
            Ok(Box::new(StatusCode::NOT_FOUND))
        }
    }
}

#[instrument(level = "trace", skip(firestore, igdb))]
pub async fn post_resolve(
    resolve: models::Resolve,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    let started = SystemTime::now();
    let response = match igdb.get(resolve.game_id).await {
        Ok(igdb_game) => igdb.resolve(firestore, igdb_game).await,
        Err(e) => Err(Status::internal(format!("igdb.get(): {e}"))),
    };
    let resp_time = SystemTime::now().duration_since(started).unwrap();

    match response {
        Ok(game_entry) => {
            info!(
                http_request.request_method = "POST",
                http_request.request_url = "/resolve",
                labels.log_type = "query_logs",
                labels.handler = "resolve",
                resolve.game_id = resolve.game_id,
                resolve.title = game_entry.name,
                resolve.latency = resp_time.as_millis(),
                "resolve '{}' ({})",
                game_entry.name,
                resolve.game_id,
            );
            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!(
                http_request.request_method = "POST",
                http_request.request_url = "/resolve",
                labels.log_type = "query_logs",
                labels.handler = "resolve",
                resolve.game_id = resolve.game_id,
                resolve.latency = resp_time.as_millis(),
                resolve.error = e.to_string(),
                "resolve: {}",
                resolve.game_id,
            );
            Ok(StatusCode::NOT_FOUND)
        }
    }
}

#[instrument(
    level = "trace",
    skip(match_op, firestore, igdb),
    fields(
        title = %match_op.store_entry.title,
    )
)]
pub async fn post_match(
    user_id: String,
    match_op: models::MatchOp,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    let started = SystemTime::now();
    let match_op_clone = match_op.clone();
    let manager = LibraryManager::new(&user_id, firestore);
    let response = match (match_op.game_entry, match_op.unmatch_entry) {
        // Match StoreEntry to GameEntry and add in Library.
        (Some(game_entry), None) => match manager.get_digest(igdb, game_entry.id).await {
            Ok(digests) => match manager.create_library_entry(match_op.store_entry, digests) {
                Ok(()) => Ok(()),
                Err(e) => Err(Status::internal(format!("create_library_entry(): {e}"))),
            },
            Err(e) => Err(Status::not_found(format!("get_digest(): {e}"))),
        },
        // Remove StoreEntry from Library.
        (None, Some(_library_entry)) => {
            match manager
                .unmatch_game(match_op.store_entry, match_op.delete_unmatched)
                .await
            {
                Ok(()) => Ok(()),
                Err(e) => Err(Status::internal(format!("unmatch_game(): {e}"))),
            }
        }
        // Match StoreEntry with a different GameEntry.
        (Some(game_entry), Some(_library_entry)) => {
            match manager
                .rematch_game(igdb, match_op.store_entry, game_entry.id)
                .await
            {
                Ok(()) => Ok(()),
                Err(e) => Err(Status::internal(format!("rematch_game(): {e}"))),
            }
        }
        // Unexpected request.
        (None, None) => Err(Status::invalid_argument(
            "Missing both game_entry and unmatch_entry args.",
        )),
    };
    let resp_time = SystemTime::now().duration_since(started).unwrap();

    let op = match (match_op_clone.game_entry, match_op_clone.unmatch_entry) {
        (Some(_), None) => "match",
        (None, Some(_)) => "unmatch",
        (Some(_), Some(_)) => "rematch",
        (None, None) => "bad_request",
    };

    match response {
        Ok(()) => {
            info!(
                http_request.request_method = "POST",
                http_request.request_url = "/library/_/match",
                labels.log_type = "query_logs",
                labels.handler = "match",
                matchOp.user_id = user_id,
                matchOp.operation = op,
                matchOp.store_entry_id = match_op_clone.store_entry.id,
                matchOp.store_entry_title = match_op_clone.store_entry.title,
                matchOp.store_entry_storefront = match_op_clone.store_entry.storefront_name,
                matchOp.latency = resp_time.as_millis(),
                "{op} '{}'",
                match_op_clone.store_entry.title,
            );
            Ok(StatusCode::OK)
        }
        Err(Status::NotFound(e)) => {
            error!(
                http_request.request_method = "POST",
                http_request.request_url = "/library/{user_id}/match",
                labels.log_type = "query_logs",
                labels.handler = "match",
                matchOp.user_id = user_id,
                matchOp.operation = op,
                matchOp.store_entry_id = match_op_clone.store_entry.id,
                matchOp.store_entry_title = match_op_clone.store_entry.title,
                matchOp.store_entry_storefront = match_op_clone.store_entry.storefront_name,
                matchOp.latency = resp_time.as_millis(),
                matchOp.error = e.to_string(),
                "{op} '{}'",
                match_op_clone.store_entry.title,
            );
            Ok(StatusCode::NOT_FOUND)
        }
        Err(Status::InvalidArgument(e)) => {
            error!(
                http_request.request_method = "POST",
                http_request.request_url = "/library/{user_id}/match",
                labels.log_type = "query_logs",
                labels.handler = "match",
                matchOp.user_id = user_id,
                matchOp.operation = op,
                matchOp.store_entry_id = match_op_clone.store_entry.id,
                matchOp.store_entry_title = match_op_clone.store_entry.title,
                matchOp.store_entry_storefront = match_op_clone.store_entry.storefront_name,
                matchOp.latency = resp_time.as_millis(),
                matchOp.error = e.to_string(),
                "{op} '{}'",
                match_op_clone.store_entry.title,
            );
            Ok(StatusCode::BAD_REQUEST)
        }
        Err(e) => {
            error!(
                http_request.request_method = "POST",
                http_request.request_url = "/library/{user_id}/match",
                labels.log_type = "query_logs",
                labels.handler = "match",
                matchOp.user_id = user_id,
                matchOp.operation = op,
                matchOp.store_entry_id = match_op_clone.store_entry.id,
                matchOp.store_entry_title = match_op_clone.store_entry.title,
                matchOp.store_entry_storefront = match_op_clone.store_entry.storefront_name,
                matchOp.latency = resp_time.as_millis(),
                matchOp.error = e.to_string(),
                "{op} '{}'",
                match_op_clone.store_entry.title,
            );
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument(level = "trace", skip(firestore))]
pub async fn post_wishlist(
    user_id: String,
    wishlist: models::WishlistOp,
    firestore: Arc<Mutex<FirestoreApi>>,
) -> Result<impl warp::Reply, Infallible> {
    let started = SystemTime::now();
    let manager = LibraryManager::new(&user_id, firestore);
    let wishlist_clone = wishlist.clone();
    let response = match (wishlist.add_game, wishlist.remove_game) {
        (Some(library_entry), _) => match manager.add_to_wishlist(library_entry).await {
            Ok(()) => Ok(wishlist_clone.add_game.as_ref().unwrap().id),
            Err(e) => Err(Status::internal(format!("add_to_wishlist(): {e}"))),
        },
        (_, Some(game_id)) => match manager.remove_from_wishlist(game_id).await {
            Ok(()) => Ok(game_id),
            Err(e) => Err(Status::internal(format!("remove_from_wishlist(): {e}"))),
        },
        _ => Err(Status::invalid_argument(
            "Missing both add_game and remove_game arguments.",
        )),
    };
    let resp_time = SystemTime::now().duration_since(started).unwrap();

    let op = match (wishlist_clone.add_game, wishlist_clone.remove_game) {
        (Some(_), _) => "add_to_wishlist",
        (_, Some(_)) => "remove_from_wishlist",
        _ => "bad_request",
    };
    match response {
        Ok(game_id) => {
            info!(
                http_request.request_method = "POST",
                http_request.request_url = "/library/_/wishlist",
                labels.log_type = "query_logs",
                labels.handler = "wishlist",
                wishlist.user_id = user_id,
                wishlist.operation = op,
                wishlist.game_id = game_id,
                wishlist.latency = resp_time.as_millis(),
                "{op} '{game_id}'",
            );
            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!(
                http_request.request_method = "POST",
                http_request.request_url = "/library/_/wishlist",
                labels.log_type = "query_logs",
                labels.handler = "wishlist",
                wishlist.user_id = user_id,
                wishlist.operation = op,
                wishlist.latency = resp_time.as_millis(),
                wishlist.error = e.to_string(),
                "{op}",
            );
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument(level = "trace", skip(firestore))]
pub async fn post_unlink(
    user_id: String,
    unlink: models::Unlink,
    firestore: Arc<Mutex<FirestoreApi>>,
) -> Result<impl warp::Reply, Infallible> {
    let started = SystemTime::now();
    // Remove storefront credentials from UserData.
    let mut response = match User::new(Arc::clone(&firestore), &user_id) {
        Ok(mut user) => match user.remove_storefront(&unlink.storefront_id) {
            Ok(()) => Ok(()),
            Err(e) => Err(Status::internal(format!("remove_storefront(): {e}"))),
        },
        Err(e) => Err(e),
    };

    if response.is_ok() {
        // Remove storefront library entries.
        let manager = LibraryManager::new(&user_id, firestore);
        response = manager.remove_storefront(&unlink.storefront_id).await;
    }
    let resp_time = SystemTime::now().duration_since(started).unwrap();

    match response {
        Ok(()) => {
            info!(
                http_request.request_method = "POST",
                http_request.request_url = "/library/_/unlink",
                labels.log_type = "query_logs",
                labels.handler = "unlink",
                unlink.user_id = user_id,
                unlink.storefront = unlink.storefront_id,
                unlink.latency = resp_time.as_millis(),
                "unlink",
            );
            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!(
                http_request.request_method = "POST",
                http_request.request_url = "/library/_/unlink",
                labels.log_type = "query_logs",
                labels.handler = "unlink",
                unlink.user_id = user_id,
                unlink.storefront = unlink.storefront_id,
                unlink.latency = resp_time.as_millis(),
                unlink.error = e.to_string(),
                "unlink",
            );
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument(level = "trace", skip(api_keys, firestore, igdb))]
pub async fn post_sync(
    user_id: String,
    api_keys: Arc<util::keys::Keys>,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let started = SystemTime::now();

    let store_entries = match User::new(Arc::clone(&firestore), &user_id) {
        Ok(mut user) => match user.sync_accounts(&api_keys).await {
            Ok(entries) => entries,
            Err(e) => {
                return Ok(log_sync_err(
                    &user_id,
                    "sync",
                    SystemTime::now().duration_since(started).unwrap(),
                    e,
                ))
            }
        },
        Err(e) => {
            return Ok(log_sync_err(
                &user_id,
                "sync",
                SystemTime::now().duration_since(started).unwrap(),
                e,
            ))
        }
    };

    let manager = LibraryManager::new(&user_id, firestore);
    let report = match manager.recon_store_entries(store_entries, igdb).await {
        Ok(report) => report,
        Err(e) => {
            return Ok(log_sync_err(
                &user_id,
                "sync",
                SystemTime::now().duration_since(started).unwrap(),
                e,
            ))
        }
    };

    let resp_time = SystemTime::now().duration_since(started).unwrap();

    info!(
        http_request.request_method = "POST",
        http_request.request_url = format!("/library/_/sync"),
        labels.log_type = "query_logs",
        labels.handler = "sync",
        sync.user_id = user_id,
        sync.report = format!("{:?}", report),
        sync.latency = resp_time.as_millis(),
        "sync",
    );

    let resp: Box<dyn warp::Reply> = Box::new(warp::reply::json(&report));
    Ok(resp)
}

#[instrument(level = "trace", skip(upload, firestore, igdb))]
pub async fn post_upload(
    user_id: String,
    upload: models::Upload,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let started = SystemTime::now();

    let manager = LibraryManager::new(&user_id, firestore);
    let report = match manager.recon_store_entries(upload.entries, igdb).await {
        Ok(report) => report,
        Err(e) => {
            return Ok(log_sync_err(
                &user_id,
                "upload",
                SystemTime::now().duration_since(started).unwrap(),
                e,
            ))
        }
    };

    let resp_time = SystemTime::now().duration_since(started).unwrap();

    info!(
        http_request.request_method = "POST",
        http_request.request_url = format!("/library/_/upload"),
        labels.log_type = "query_logs",
        labels.handler = "upload",
        sync.user_id = user_id,
        sync.report = format!("{:?}", report),
        sync.latency = resp_time.as_millis(),
        "upload",
    );

    let resp: Box<dyn warp::Reply> = Box::new(warp::reply::json(&report));
    Ok(resp)
}

#[instrument(level = "trace")]
pub async fn get_images(
    resolution: String,
    image: String,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    debug!("GET /images/{resolution}/{image}");

    let uri = format!("{IGDB_IMAGES_URL}/{resolution}/{image}");
    let resp = match reqwest::Client::new().get(&uri).send().await {
        Ok(resp) => resp,
        Err(err) => {
            warn! {"{err}"}
            return Ok(Box::new(StatusCode::NOT_FOUND));
        }
    };

    if resp.status() != StatusCode::OK {
        warn! {"Failed to retrieve image: {uri} \nerr: {}", resp.status()}
        return Ok(Box::new(resp.status()));
    }

    match resp.bytes().await {
        Ok(bytes) => Ok(Box::new(bytes.to_vec())),
        Err(_) => Ok(Box::new(StatusCode::NOT_FOUND)),
    }
}

const IGDB_IMAGES_URL: &str = "https://images.igdb.com/igdb/image/upload";

fn log_sync_err(
    user_id: &str,
    handler_name: &str,
    resp_time: Duration,
    e: Status,
) -> Box<dyn warp::Reply> {
    error!(
        http_request.request_method = "POST",
        http_request.request_url = format!("/library/_/{handler_name}"),
        labels.log_type = "query_logs",
        labels.handler = handler_name,
        sync.user_id = user_id,
        sync.latency = resp_time.as_millis(),
        sync.error = e.to_string(),
        "get_image",
    );

    let status: Box<dyn warp::Reply> = Box::new(match e {
        Status::NotFound(_) => StatusCode::NOT_FOUND,
        Status::InvalidArgument(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    });
    status
}
