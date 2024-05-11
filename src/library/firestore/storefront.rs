use crate::{api::FirestoreApi, documents::StoreEntry, documents::Storefront, Status};
use std::collections::HashSet;
use tracing::instrument;

/// Returns all store entries owned by user.
///
/// Reads `users/{user_id}/games/storefront` document in Firestore.
#[instrument(name = "storefront::read", level = "trace", skip(firestore, user_id))]
pub async fn read(firestore: &FirestoreApi, user_id: &str) -> Result<Storefront, Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    let doc: Option<Storefront> = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(GAMES)
        .parent(&parent_path)
        .obj()
        .one(STOREFRONT_DOC)
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!("storefronts doc not found"))),
    }
}

/// Writes the Storefront doc containing games owned by user.
///
/// Writes `users/{user_id}/games/storefront` document in Firestore.
#[instrument(
    name = "storefront::write",
    level = "trace",
    skip(firestore, user_id, storefront)
)]
pub async fn write(
    firestore: &FirestoreApi,
    user_id: &str,
    storefront: &Storefront,
) -> Result<(), Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    firestore
        .db()
        .fluent()
        .update()
        .in_col(GAMES)
        .document_id(STOREFRONT_DOC)
        .parent(&parent_path)
        .object(storefront)
        .execute()
        .await?;
    Ok(())
}

/// Returns input StoreEntries that are not already contained in user's
/// Storefront document.
///
/// Reads `users/{user_id}/games/storefront` document in Firestore.
#[instrument(
    name = "storefront::diff_entries",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn diff_entries(
    firestore: &FirestoreApi,
    user_id: &str,
    mut store_entries: Vec<StoreEntry>,
) -> Result<Vec<StoreEntry>, Status> {
    let game_ids = get_ids(firestore, user_id).await?;
    store_entries.retain(|entry| !game_ids.contains(&entry.id));

    Ok(store_entries)
}

/// Returns set of store game ids owned by user from specified storefront.
///
/// Reads `users/{user_id}/games/storefront` document in Firestore.
#[instrument(
    name = "storefront::get_ids",
    level = "trace",
    skip(firestore, user_id)
)]
async fn get_ids(firestore: &FirestoreApi, user_id: &str) -> Result<HashSet<String>, Status> {
    match read(firestore, user_id).await {
        Ok(doc) => Ok(HashSet::from_iter(doc.games.into_iter().map(|e| e.id))),
        Err(Status::NotFound(_)) => Ok(HashSet::default()),
        Err(status) => Err(status),
    }
}

/// Deletes all StoreEntries from specified storefront.
///
/// Reads/Writes `users/{user_id}/games/storefront` document in Firestore.
#[instrument(name = "storefront::delete", level = "trace", skip(firestore, user_id))]
pub async fn remove_store(
    firestore: &FirestoreApi,
    user_id: &str,
    storefront_name: &str,
) -> Result<(), Status> {
    let mut storefront = read(firestore, user_id).await?;
    storefront
        .games
        .retain(|entry| entry.storefront_name != *storefront_name);
    write(firestore, user_id, &storefront).await
}

/// Add StoreEntry to the user's Storefront document.
///
/// Reads/Writes `users/{user_id}/games/storefront` document in Firestore.
#[instrument(
    name = "storefront::add_entries",
    level = "trace",
    skip(firestore, user_id, store_entries)
)]
pub async fn add_entries(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entries: Vec<StoreEntry>,
) -> Result<(), Status> {
    let mut storefront = read(firestore, user_id).await?;
    storefront.games.extend(store_entries.into_iter());
    write(firestore, user_id, &storefront).await
}

/// Remove a StoreEntry from its Storefront.
///
/// Reads/writes `users/{user}/storefronts/{storefront_name}` document in
/// Firestore.
#[instrument(name = "storefront::remove", level = "trace", skip(firestore, user_id))]
pub async fn remove_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entry: &StoreEntry,
) -> Result<(), Status> {
    let mut storefront = read(firestore, user_id).await?;
    storefront
        .games
        .retain(|e| e.id != store_entry.id || e.storefront_name != store_entry.storefront_name);
    write(firestore, user_id, &storefront).await
}

const USERS: &str = "users";
const GAMES: &str = "games";
const STOREFRONT_DOC: &str = "storefront";
