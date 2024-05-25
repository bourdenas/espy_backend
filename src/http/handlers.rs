use crate::{
    api::{FirestoreApi, IgdbApi, IgdbSearch},
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
    let igdb_search = IgdbSearch::new(igdb);
    match igdb_search
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

#[instrument(level = "trace", skip(firestore))]
pub async fn post_update(
    user_id: String,
    update: models::UpdateOp,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    let event = UpdateEvent::new(&update);

    let game_entry = match games::read(&firestore, update.game_id).await {
        Ok(game_entry) => game_entry,
        Err(status) => {
            event.log_error(&user_id, status);
            return Ok(StatusCode::NOT_FOUND);
        }
    };

    let manager = LibraryManager::new(&user_id);
    match manager.update_game(firestore, game_entry).await {
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

    let game_entry = match match_op.game_entry {
        Some(game_entry) => match games::read(&firestore, game_entry.id).await {
            Ok(game_entry) => Some(game_entry),
            Err(Status::NotFound(_)) => {
                // TODO: move inside igdb service.
                let igdb_game = match igdb.get(game_entry.id).await {
                    Ok(igdb_game) => igdb_game,
                    Err(status) => {
                        event.log_error(&user_id, status);
                        return Ok(StatusCode::NOT_FOUND);
                    }
                };
                match igdb.resolve(Arc::clone(&firestore), igdb_game).await {
                    Ok(digest) => Some(digest),
                    Err(status) => {
                        event.log_error(&user_id, status);
                        return Ok(StatusCode::NOT_FOUND);
                    }
                }
            }
            Err(status) => {
                event.log_error(&user_id, status);
                return Ok(StatusCode::NOT_FOUND);
            }
        },
        None => None,
    };

    let manager = LibraryManager::new(&user_id);
    let status = match (game_entry, match_op.unmatch_entry) {
        // Match StoreEntry to GameEntry and add in Library.
        (Some(game_entry), None) => {
            manager
                .create_library_entry(firestore, match_op.store_entry, game_entry)
                .await
        }
        // Remove StoreEntry from Library.
        (None, Some(_)) => {
            manager
                .unmatch_game(firestore, match_op.store_entry, match_op.delete_unmatched)
                .await
        }
        // Match StoreEntry with a different GameEntry.
        (Some(game_entry), Some(_library_entry)) => {
            manager
                .rematch_game(firestore, match_op.store_entry, game_entry)
                .await
        }
        // Bad request, at least one must be present.
        (None, None) => Err(Status::invalid_argument(
            "Missing both game_entry and unmatch_entry args.",
        )),
    };

    match status {
        Ok(()) => {
            event.log(&user_id);
            Ok(StatusCode::OK)
        }
        Err(Status::InvalidArgument(status)) => {
            event.log_error(&user_id, Status::invalid_argument(status));
            Ok(StatusCode::BAD_REQUEST)
        }
        Err(status) => {
            event.log_error(&user_id, status);
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
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
) -> Result<impl warp::Reply, Infallible> {
    let event = SyncEvent::new();

    let store_entries = match User::fetch(Arc::clone(&firestore), &user_id).await {
        Ok(mut user) => user.sync_accounts(&api_keys).await,
        Err(status) => Err(status),
    };

    let store_entries = match store_entries {
        Ok(store_entries) => store_entries,
        Err(status) => {
            event.log_error(&user_id, status);
            return Ok(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let manager = LibraryManager::new(&user_id);
    match manager
        .batch_recon_store_entries(firestore, igdb, store_entries)
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
