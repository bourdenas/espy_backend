use crate::{
    api::{FirestoreApi, IgdbApi},
    games::ReconReport,
    http::models,
    library::{firestore::games, LibraryManager, User},
    util, Status,
};
use std::{convert::Infallible, sync::Arc};
use tracing::{info, instrument, warn};
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
    firestore: Arc<FirestoreApi>,
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

#[instrument(level = "trace", skip(firestore))]
pub async fn post_delete(
    resolve: models::Resolve,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    match games::delete(&firestore, resolve.game_id).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[instrument(level = "trace", skip(firestore, igdb))]
pub async fn post_update(
    user_id: String,
    update: models::UpdateOp,
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    let event = UpdateEvent::new(&update);

    let manager = LibraryManager::new(&user_id);
    match manager.update_game(firestore, igdb, update.game_id).await {
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
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<impl warp::Reply, Infallible> {
    let event = MatchEvent::new(match_op.clone());

    let manager = LibraryManager::new(&user_id);
    match (match_op.game_entry, match_op.unmatch_entry) {
        // Match StoreEntry to GameEntry and add in Library.
        (Some(game_entry), None) => {
            match manager
                .get_digest(Arc::clone(&firestore), igdb, game_entry.id)
                .await
            {
                Ok(digests) => match manager
                    .create_library_entry(firestore, match_op.store_entry, digests)
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
                },
                Err(status) => {
                    event.log_error(&user_id, status);
                    Ok(StatusCode::NOT_FOUND)
                }
            }
        }
        // Remove StoreEntry from Library.
        (None, Some(_library_entry)) => {
            match manager
                .unmatch_game(firestore, match_op.store_entry, match_op.delete_unmatched)
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
                .rematch_game(firestore, igdb, match_op.store_entry, game_entry.id)
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
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    let event = WishlistEvent::new(wishlist.clone());

    let manager = LibraryManager::new(&user_id);
    match (wishlist.add_game, wishlist.remove_game) {
        (Some(library_entry), _) => match manager.add_to_wishlist(firestore, library_entry).await {
            Ok(()) => {
                event.log(&user_id);
                Ok(StatusCode::OK)
            }
            Err(status) => {
                event.log_error(&user_id, status);
                Ok(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        (_, Some(game_id)) => match manager.remove_from_wishlist(firestore, game_id).await {
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
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    let event = UnlinkEvent::new(&unlink);

    match User::fetch(Arc::clone(&firestore), &user_id).await {
        // Remove storefront credentials from UserData.
        Ok(mut user) => match user.remove_storefront(&unlink.storefront_id).await {
            Ok(()) => {
                // Remove storefront library entries.
                let manager = LibraryManager::new(&user_id);
                match manager
                    .remove_storefront(firestore, &unlink.storefront_id)
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
    firestore: Arc<FirestoreApi>,
    igdb: Arc<IgdbApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let event = SyncEvent::new();

    let store_entries = match User::fetch(Arc::clone(&firestore), &user_id).await {
        Ok(mut user) => match user.sync_accounts(&api_keys).await {
            Ok(entries) => entries,
            Err(status) => {
                event.log_error(&user_id, status);
                return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
            }
        },
        Err(status) => {
            event.log_error(&user_id, status);
            return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
        }
    };

    let manager = LibraryManager::new(&user_id);
    let report = match manager
        .batch_recon_store_entries(firestore, igdb, store_entries)
        .await
    {
        Ok(()) => ReconReport {
            lines: vec!["Done".to_owned()],
        },
        Err(status) => {
            event.log_error(&user_id, status);
            return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
        }
    };

    event.log(&user_id, &report);
    let resp: Box<dyn warp::Reply> = Box::new(warp::reply::json(&report));
    Ok(resp)
}

#[instrument(level = "trace")]
pub async fn get_images(
    resolution: String,
    image: String,
) -> Result<Box<dyn warp::Reply>, Infallible> {
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
