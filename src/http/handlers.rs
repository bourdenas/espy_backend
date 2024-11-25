use crate::{
    api::{CompanyNormalizer, FirestoreApi},
    documents::GameEntry,
    http::models,
    library::{self, LibraryManager, User},
    log_request,
    logging::LogHttpRequest,
    resolver::ResolveApi,
    util, Status,
};
use std::{convert::Infallible, sync::Arc};
use tracing::{info, instrument, warn};
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

#[instrument(name = "search", level = "info", skip(resolver))]
pub async fn post_search(
    search: models::Search,
    resolver: Arc<ResolveApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    match resolver
        .search(search.title.clone(), search.base_game_only)
        .await
    {
        Ok(candidates) => {
            log_request!(LogHttpRequest::search(search, &candidates));
            Ok(Box::new(warp::reply::json(&candidates)))
        }
        Err(status) => {
            log_request!(LogHttpRequest::search_err(search, status));
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[instrument(name = "company_fetch", level = "info", skip(firestore))]
pub async fn post_company_fetch(
    company_fetch: models::CompanyFetch,
    firestore: Arc<FirestoreApi>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let slug = CompanyNormalizer::slug(&company_fetch.name);
    match library::firestore::companies::search(&firestore, &slug).await {
        Ok(companies) => {
            log_request!(LogHttpRequest::company_search(company_fetch, &companies));
            Ok(Box::new(warp::reply::json(&companies)))
        }
        Err(status) => {
            log_request!(LogHttpRequest::company_search_err(company_fetch, status));
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[instrument(name = "resolve", level = "info", skip(firestore, resolver))]
pub async fn post_resolve(
    resolve: models::Resolve,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
) -> Result<impl warp::Reply, Infallible> {
    match retrieve_and_store(&firestore, &resolver, resolve.game_id).await {
        Ok(game_entry) => {
            log_request!(LogHttpRequest::resolve(resolve, game_entry));
            Ok(StatusCode::OK)
        }
        Err(Status::NotFound(msg)) => {
            log_request!(LogHttpRequest::resolve_err(resolve, Status::not_found(msg)));
            Ok(StatusCode::NOT_FOUND)
        }
        Err(status) => {
            log_request!(LogHttpRequest::resolve_err(resolve, status));
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument(name = "delete", level = "info", skip(firestore))]
pub async fn post_delete(
    resolve: models::Resolve,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    match library::firestore::games::delete(&firestore, resolve.game_id).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[instrument(name = "update", level = "info", skip(firestore))]
pub async fn post_update(
    user_id: String,
    update: models::UpdateOp,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    let game_entry = match library::firestore::games::read(&firestore, update.game_id).await {
        Ok(game_entry) => game_entry,
        Err(status) => {
            log_request!(LogHttpRequest::update(update, status));
            return Ok(StatusCode::NOT_FOUND);
        }
    };

    let manager = LibraryManager::new(&user_id);
    match manager.update_game(firestore, game_entry).await {
        Ok(()) => {
            log_request!(LogHttpRequest::update(update, Status::Ok));
            Ok(StatusCode::OK)
        }
        Err(status) => {
            log_request!(LogHttpRequest::update(update, status));
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument(
    name = "match",
    level = "info",
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
    let game_entry = match match_op.game_id {
        Some(game_id) => match library::firestore::games::read(&firestore, game_id).await {
            Ok(game_entry) => Some(game_entry),
            Err(Status::NotFound(_)) => {
                match retrieve_and_store(&firestore, &resolver, game_id).await {
                    Ok(game_entry) => Some(game_entry),
                    Err(status) => {
                        log_request!(LogHttpRequest::match_game(match_op, status));
                        return Ok(StatusCode::NOT_FOUND);
                    }
                }
            }
            Err(status) => {
                log_request!(LogHttpRequest::match_game(match_op, status));
                return Ok(StatusCode::NOT_FOUND);
            }
        },
        None => None,
    };

    let request = match_op.clone();
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
            log_request!(LogHttpRequest::match_game(request, Status::Ok));
            Ok(StatusCode::OK)
        }
        Err(Status::InvalidArgument(msg)) => {
            log_request!(LogHttpRequest::match_game(
                request,
                Status::invalid_argument(msg),
            ));
            Ok(StatusCode::BAD_REQUEST)
        }
        Err(status) => {
            log_request!(LogHttpRequest::match_game(request, status));
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument(name = "wishlist", level = "info", skip(firestore))]
pub async fn post_wishlist(
    user_id: String,
    wishlist: models::WishlistOp,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    let request = wishlist.clone();
    let manager = LibraryManager::new(&user_id);
    match (wishlist.add_game, wishlist.remove_game) {
        (Some(library_entry), _) => match manager.add_to_wishlist(firestore, library_entry).await {
            Ok(()) => {
                log_request!(LogHttpRequest::wishlist(request, Status::Ok));
                Ok(StatusCode::OK)
            }
            Err(status) => {
                log_request!(LogHttpRequest::wishlist(request, status));
                Ok(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        (_, Some(game_id)) => match manager.remove_from_wishlist(firestore, game_id).await {
            Ok(()) => {
                log_request!(LogHttpRequest::wishlist(request, Status::Ok));
                Ok(StatusCode::OK)
            }
            Err(status) => {
                log_request!(LogHttpRequest::wishlist(request, status));
                Ok(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        _ => {
            log_request!(LogHttpRequest::wishlist(
                request,
                Status::invalid_argument("Missing both add_game and remove_game arguments.")
            ));
            Ok(StatusCode::BAD_REQUEST)
        }
    }
}

#[instrument(name = "unlink", level = "info", skip(firestore))]
pub async fn post_unlink(
    user_id: String,
    unlink: models::Unlink,
    firestore: Arc<FirestoreApi>,
) -> Result<impl warp::Reply, Infallible> {
    let request = unlink.clone();
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
                        log_request!(LogHttpRequest::unlink(request, Status::Ok));
                        Ok(StatusCode::OK)
                    }
                    Err(status) => {
                        log_request!(LogHttpRequest::unlink(request, status));
                        Ok(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }
            Err(status) => {
                log_request!(LogHttpRequest::unlink(request, status));
                Ok(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        Err(status) => {
            log_request!(LogHttpRequest::unlink(request, status));
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument(name = "sync", level = "info", skip(api_keys, firestore, resolver))]
pub async fn post_sync(
    user_id: String,
    api_keys: Arc<util::keys::Keys>,
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
) -> Result<impl warp::Reply, Infallible> {
    let store_entries = match User::fetch(Arc::clone(&firestore), &user_id).await {
        Ok(mut user) => user.sync_accounts(&api_keys).await,
        Err(status) => Err(status),
    };

    let store_entries = match store_entries {
        Ok(store_entries) => store_entries,
        Err(status) => {
            log_request!(LogHttpRequest::sync(status));
            return Ok(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let manager = LibraryManager::new(&user_id);
    match manager
        .add_in_library(firestore, resolver, store_entries)
        .await
    {
        Ok(()) => {
            log_request!(LogHttpRequest::sync(Status::Ok));
            Ok(StatusCode::OK)
        }
        Err(status) => {
            log_request!(LogHttpRequest::sync(status));
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument(level = "info")]
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
