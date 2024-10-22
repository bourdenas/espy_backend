use crate::{
    api::{CompanyNormalizer, FirestoreApi},
    documents::GameEntry,
    http::models,
    library::{self, LibraryManager, User},
    resolver::ResolveApi,
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

#[instrument(level = "trace", skip(resolver))]
pub async fn post_search(
    search: models::Search,
    resolver: Arc<ResolveApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let event = SearchEvent::new(search.clone());
    match resolver.search(search.title, search.base_game_only).await {
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

#[instrument(level = "trace", skip(firestore))]
pub async fn post_company_fetch(
    company_fetch: models::CompanyFetch,
    firestore: Arc<FirestoreApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let event = CompanyFetchEvent::new(company_fetch.clone());

    let slug = CompanyNormalizer::slug(&company_fetch.name);
    match library::firestore::companies::fetch(&firestore, &slug).await {
        Ok(companies) => {
            event.log(&slug, &companies);
            Ok(Box::new(warp::reply::json(&companies)))
        }
        Err(status) => {
            event.log_error(status);
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[instrument(level = "trace", skip(firestore, resolver))]
pub async fn post_resolve(
    resolve: models::Resolve,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
) -> Result<impl warp::Reply, Infallible> {
    let event = ResolveEvent::new(resolve.clone());

    match retrieve_and_store(&firestore, &resolver, resolve.game_id).await {
        Ok(game_entry) => {
            event.log(game_entry);
            Ok(StatusCode::OK)
        }
        Err(Status::NotFound(msg)) => {
            event.log_error(Status::not_found(msg));
            Ok(StatusCode::NOT_FOUND)
        }
        Err(status) => {
            event.log_error(status);
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument(level = "trace", skip(firestore))]
pub async fn post_delete(
    resolve: models::Resolve,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    match library::firestore::games::delete(&firestore, resolve.game_id).await {
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

    let game_entry = match library::firestore::games::read(&firestore, update.game_id).await {
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
    skip(match_op, firestore, resolver),
    fields(
        title = %match_op.store_entry.title,
    )
)]
pub async fn post_match(
    user_id: String,
    match_op: models::MatchOp,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
) -> Result<impl warp::Reply, Infallible> {
    let event = MatchEvent::new(match_op.clone());

    let game_entry = match match_op.game_id {
        Some(game_id) => match library::firestore::games::read(&firestore, game_id).await {
            Ok(game_entry) => Some(game_entry),
            Err(Status::NotFound(_)) => {
                match retrieve_and_store(&firestore, &resolver, game_id).await {
                    Ok(game_entry) => Some(game_entry),
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

#[instrument(level = "trace", skip(api_keys, firestore, resolver))]
pub async fn post_sync(
    user_id: String,
    api_keys: Arc<util::keys::Keys>,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
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
        .batch_recon_store_entries(firestore, resolver, store_entries)
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
pub async fn get_images(uri: String) -> Result<Box<dyn warp::Reply>, Infallible> {
    let resp = match reqwest::Client::new().get(&uri).send().await {
        Ok(resp) => resp,
        Err(err) => {
            warn!("{err}");
            return Ok(Box::new(StatusCode::NOT_FOUND));
        }
    };

    if !resp.status().is_success() {
        warn!("Failed to retrieve image: {uri} \nerr: {}", resp.status());
        return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
    }

    match resp.bytes().await {
        Ok(bytes) => Ok(Box::new(bytes.to_vec())),
        Err(_) => Ok(Box::new(StatusCode::NOT_FOUND)),
    }
}

async fn retrieve_and_store(
    firestore: &FirestoreApi,
    resolver: &ResolveApi,
    id: u64,
) -> Result<GameEntry, Status> {
    match resolver.retrieve(id).await {
        Ok(mut game_entry) => {
            match library::firestore::games::write(&firestore, &mut game_entry).await {
                Ok(()) => Ok(game_entry),
                Err(status) => Err(status),
            }
        }
        Err(status) => Err(status),
    }
}
