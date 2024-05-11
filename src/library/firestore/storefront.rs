use crate::{api::FirestoreApi, documents::StoreEntry, documents::Storefront, Status};
use std::collections::{HashMap, HashSet};
use tracing::instrument;

/// Returns all store entries owned by user from specified storefront.
///
/// Reads `users/{user_id}/storefronts/{storefront_name}` document in Firestore.
#[instrument(name = "storefront::read", level = "trace", skip(firestore, user_id))]
pub async fn read(
    firestore: &FirestoreApi,
    user_id: &str,
    storefront: &str,
) -> Result<Storefront, Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    let doc: Option<Storefront> = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(STOREFRONTS)
        .parent(&parent_path)
        .obj()
        .one(storefront)
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Storefront '{storefront}' doc not found."
        ))),
    }
}

/// Writes all store game ids owned by user from specified storefront.
///
/// Writes `users/{user_id}/storefronts/{storefront_name}` document in
/// Firestore.
#[instrument(
    name = "storefront::write",
    level = "trace",
    skip(firestore, user_id, storefront)
    fields(
        storefront = %storefront.name
    )
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
        .in_col(STOREFRONTS)
        .document_id(&storefront.name)
        .parent(&parent_path)
        .object(storefront)
        .execute()
        .await?;
    Ok(())
}

/// Deletes a storefront record from user's library.
///
/// Deletes `users/{user_id}/storefronts/{storefront}` document in Firestore.
#[instrument(name = "storefront::delete", level = "trace", skip(firestore, user_id))]
pub async fn delete(
    firestore: &FirestoreApi,
    user_id: &str,
    storefront: &str,
) -> Result<(), Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    firestore
        .db()
        .fluent()
        .delete()
        .from(STOREFRONTS)
        .parent(&parent_path)
        .document_id(storefront)
        .execute()
        .await?;
    Ok(())
}

/// Returns input StoreEntries that are not already contained in user's
/// Storefront document.
///
/// Reads `users/{user_id}/storefronts/{storefront_name}` document in Firestore.
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
    let storefront_name = match store_entries.first() {
        Some(entry) => &entry.storefront_name,
        None => return Ok(vec![]),
    };

    let game_ids = get_ids(firestore, user_id, storefront_name).await?;
    store_entries.retain(|entry| !game_ids.contains(&entry.id));

    Ok(store_entries)
}

/// Returns set of store game ids owned by user from specified storefront.
///
/// Reads `users/{user_id}/storefronts/{storefront_name}` document in Firestore.
#[instrument(
    name = "storefront::get_ids",
    level = "trace",
    skip(firestore, user_id)
)]
async fn get_ids(
    firestore: &FirestoreApi,
    user_id: &str,
    storefront: &str,
) -> Result<HashSet<String>, Status> {
    match read(firestore, user_id, storefront).await {
        Ok(doc) => Ok(HashSet::from_iter(doc.games.into_iter().map(|e| e.id))),
        Err(Status::NotFound(_)) => Ok(HashSet::default()),
        Err(status) => Err(status),
    }
}

/// Add StoreEntry ids to the user's Storefront document.
///
/// Reads/writes `users/{user_id}/storefronts/{storefront_name}` document in
/// Firestore.
#[instrument(
    name = "storefront::add_entries",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn add_entries(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entries: Vec<StoreEntry>,
) -> Result<(), Status> {
    for (name, store_entries) in group_by(store_entries) {
        let mut storefront = match read(firestore, user_id, &name).await {
            Ok(doc) => doc,
            Err(Status::NotFound(_)) => Storefront::default(),
            Err(status) => return Err(status),
        };
        storefront.games.extend(store_entries.into_iter());
        write(firestore, user_id, &storefront).await?
    }

    Ok(())
}

/// Groups StoreEntries by storefront name.
fn group_by(store_entries: Vec<StoreEntry>) -> HashMap<String, Vec<StoreEntry>> {
    let mut groups = HashMap::<String, Vec<StoreEntry>>::new();

    for entry in store_entries {
        match groups.get_mut(&entry.storefront_name) {
            Some(entries) => entries.push(entry),
            None => {
                groups.insert(entry.storefront_name.to_owned(), vec![entry]);
            }
        }
    }

    groups
}

/// Add StoreEntry id to the user's Storefront document.
///
/// Reads/writes `users/{user_id}/storefronts/{storefront_name}` document in
/// Firestore.
#[instrument(
    name = "storefront::add_entry",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn add_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entry: StoreEntry,
) -> Result<(), Status> {
    let mut storefront = match read(firestore, user_id, &store_entry.storefront_name).await {
        Ok(doc) => doc,
        Err(Status::NotFound(_)) => Storefront::default(),
        Err(status) => return Err(status),
    };
    storefront.games.push(store_entry);
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
    let mut storefront = read(firestore, user_id, &store_entry.storefront_name).await?;
    storefront.games.retain(|e| e.id != store_entry.id);
    write(firestore, user_id, &storefront).await
}

const USERS: &str = "users";
const STOREFRONTS: &str = "storefronts";
