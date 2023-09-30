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

use super::query_logs::*;

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
    let event = SearchEvent::new(&search);
    match igdb
        .search_by_title_with_cover(&search.title, search.base_game_only)
        .await
    {
        Ok(candidates) => {
            event.log(&candidates);
            Ok(Box::new(warp::reply::json(&candidates)))
        }
        Err(status) => {
            event.log_error(status);
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
    let event = ResolveEvent::new(&resolve);
    match igdb.get(resolve.game_id).await {
        Ok(igdb_game) => match igdb.resolve(firestore, igdb_game).await {
            Ok(game_entry) => {
                event.log(game_entry);
                Ok(StatusCode::OK)
            }
            Err(status) => {
                event.log_error(status);
                Ok(StatusCode::NOT_FOUND)
            }
        },
        Err(status) => {
            event.log_error(status);
            Ok(StatusCode::NOT_FOUND)
        }
    }
}

#[instrument(level = "trace", skip(firestore, igdb))]
pub async fn post_update(
    user_id: String,
    update: models::UpdateOp,
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    let event = UpdateEvent::new(&update);

    let manager = LibraryManager::new(&user_id, firestore);
    match manager.update_game(igdb, update.game_id).await {
        Ok(()) => {
            event.log(&user_id);
            Ok(StatusCode::OK)
        }
        Err(status) => {
            event.log_error(&user_id, status);
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
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
    let event = MatchEvent::new(match_op.clone());

    let manager = LibraryManager::new(&user_id, firestore);
    match (match_op.game_entry, match_op.unmatch_entry) {
        // Match StoreEntry to GameEntry and add in Library.
        (Some(game_entry), None) => match manager.get_digest(igdb, game_entry.id).await {
            Ok(digests) => match manager.create_library_entry(match_op.store_entry, digests) {
                Ok(()) => {
                    event.log(&user_id);
                    Ok(StatusCode::OK)
                }
                Err(status) => {
                    event.log_error(&user_id, status);
                    Ok(StatusCode::INTERNAL_SERVER_ERROR)
                }
            },
            Err(status) => {
                event.log_error(&user_id, status);
                Ok(StatusCode::NOT_FOUND)
            }
        },
        // Remove StoreEntry from Library.
        (None, Some(_library_entry)) => {
            match manager
                .unmatch_game(match_op.store_entry, match_op.delete_unmatched)
                .await
            {
                Ok(()) => {
                    event.log(&user_id);
                    Ok(StatusCode::OK)
                }
                Err(status) => {
                    event.log_error(&user_id, status);
                    Ok(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        // Match StoreEntry with a different GameEntry.
        (Some(game_entry), Some(_library_entry)) => {
            match manager
                .rematch_game(igdb, match_op.store_entry, game_entry.id)
                .await
            {
                Ok(()) => {
                    event.log(&user_id);
                    Ok(StatusCode::OK)
                }
                Err(status) => {
                    event.log_error(&user_id, status);
                    Ok(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        // Bad request, at least one must be present.
        (None, None) => {
            event.log_error(
                &user_id,
                Status::invalid_argument("Missing both game_entry and unmatch_entry args."),
            );
            Ok(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(level = "trace", skip(firestore))]
pub async fn post_wishlist(
    user_id: String,
    wishlist: models::WishlistOp,
    firestore: Arc<Mutex<FirestoreApi>>,
) -> Result<impl warp::Reply, Infallible> {
    let event = WishlistEvent::new(wishlist.clone());

    let manager = LibraryManager::new(&user_id, firestore);
    match (wishlist.add_game, wishlist.remove_game) {
        (Some(library_entry), _) => match manager.add_to_wishlist(library_entry).await {
            Ok(()) => {
                event.log(&user_id);
                Ok(StatusCode::OK)
            }
            Err(status) => {
                event.log_error(&user_id, status);
                Ok(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        (_, Some(game_id)) => match manager.remove_from_wishlist(game_id).await {
            Ok(()) => {
                event.log(&user_id);
                Ok(StatusCode::OK)
            }
            Err(status) => {
                event.log_error(&user_id, status);
                Ok(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        _ => {
            event.log_error(
                &user_id,
                Status::invalid_argument("Missing both add_game and remove_game arguments."),
            );
            Ok(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(level = "trace", skip(firestore))]
pub async fn post_unlink(
    user_id: String,
    unlink: models::Unlink,
    firestore: Arc<Mutex<FirestoreApi>>,
) -> Result<impl warp::Reply, Infallible> {
    let event = UnlinkEvent::new(&unlink);

    match User::new(Arc::clone(&firestore), &user_id) {
        // Remove storefront credentials from UserData.
        Ok(mut user) => match user.remove_storefront(&unlink.storefront_id) {
            Ok(()) => {
                // Remove storefront library entries.
                let manager = LibraryManager::new(&user_id, firestore);
                match manager.remove_storefront(&unlink.storefront_id).await {
                    Ok(()) => {
                        event.log(&user_id);
                        Ok(StatusCode::OK)
                    }
                    Err(status) => {
                        event.log_error(&user_id, status);
                        Ok(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }
            Err(status) => {
                event.log_error(&user_id, status);
                Ok(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        Err(status) => {
            event.log_error(&user_id, status);
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
