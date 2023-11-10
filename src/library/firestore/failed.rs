use crate::{
    api::FirestoreApi,
    documents::{FailedEntries, StoreEntry},
    Status,
};
use tracing::instrument;

#[instrument(
    name = "failed::add_entry",
    level = "trace",
    skip(firestore, user_id, store_entry),
    fields(store_entry_id = %store_entry.id),
)]
pub async fn add_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entry: StoreEntry,
) -> Result<(), Status> {
    let mut failed = read(firestore, user_id).await?;
    if add(store_entry, &mut failed) {
        write(firestore, user_id, &failed).await?;
    }
    Ok(())
}

#[instrument(
    name = "failed::remove_entry",
    level = "trace",
    skip(firestore, user_id, store_entry),
    fields(store_entry_id = %store_entry.id),
)]
pub async fn remove_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entry: &StoreEntry,
) -> Result<(), Status> {
    let mut failed = read(firestore, user_id).await?;
    if remove(store_entry, &mut failed) {
        write(firestore, user_id, &failed).await?;
    }
    Ok(())
}

#[instrument(
    name = "failed::remove_storefront",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn remove_storefront(
    firestore: &FirestoreApi,
    user_id: &str,
    storefront_id: &str,
) -> Result<(), Status> {
    let mut failed = read(firestore, user_id).await?;
    remove_store_entries(storefront_id, &mut failed);
    write(firestore, user_id, &failed).await
}

/// Adds `StoreEntry` in the failed to match entries.
///
/// Returns false if the same `StoreEntry` was already found, true otherwise.
fn add(store_entry: StoreEntry, failed: &mut FailedEntries) -> bool {
    match failed
        .entries
        .iter()
        .find(|e| e.id == store_entry.id && e.storefront_name == store_entry.storefront_name)
    {
        Some(_) => false,
        None => {
            failed.entries.push(store_entry);
            true
        }
    }
}

/// Remove `StoreEntry` from the failed to match entries.
///
/// Returns true if the `StoreEntry` was found and removed, false otherwise.
fn remove(store_entry: &StoreEntry, failed: &mut FailedEntries) -> bool {
    let original_len = failed.entries.len();
    failed
        .entries
        .retain(|e| e.id != store_entry.id || e.storefront_name != store_entry.storefront_name);

    failed.entries.len() != original_len
}

/// Remove all failed store entries from specified storefront.
fn remove_store_entries(storefront_id: &str, failed: &mut FailedEntries) {
    failed
        .entries
        .retain(|store_entry| store_entry.storefront_name != storefront_id);
}

// TODO: This should become private and move storefront cleanup logic inside this module.
#[instrument(name = "failed::read", level = "trace", skip(firestore, user_id))]
pub async fn read(firestore: &FirestoreApi, user_id: &str) -> Result<FailedEntries, Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(GAMES)
        .parent(&parent_path)
        .obj()
        .one(FAILED_DOC)
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{USERS}/{user_id}/{GAMES}/{FAILED_DOC}' was not found"
        ))),
    }
}

#[instrument(
    name = "failed::write",
    level = "trace",
    skip(firestore, user_id, failed)
)]
async fn write(
    firestore: &FirestoreApi,
    user_id: &str,
    failed: &FailedEntries,
) -> Result<(), Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    firestore
        .db()
        .fluent()
        .update()
        .in_col(GAMES)
        .document_id(FAILED_DOC)
        .parent(&parent_path)
        .object(failed)
        .execute()
        .await?;
    Ok(())
}

const USERS: &str = "users";
const GAMES: &str = "games";
const FAILED_DOC: &str = "failed";

#[cfg(test)]
mod tests {
    use super::*;

    fn new_store_entry(id: &str, storefront: &str) -> StoreEntry {
        StoreEntry {
            id: id.to_owned(),
            title: "Game Title".to_owned(),
            storefront_name: storefront.to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn add_in_empty_library() {
        let mut failed = FailedEntries { entries: vec![] };

        assert_eq!(add(new_store_entry("123", "gog"), &mut failed), true);
        assert_eq!(failed.entries.len(), 1);
    }

    #[test]
    fn add_in_non_empty_library() {
        let mut failed = FailedEntries {
            entries: vec![new_store_entry("213", "gog")],
        };

        assert_eq!(add(new_store_entry("123", "gog"), &mut failed), true);
        assert_eq!(failed.entries.len(), 2);
    }

    #[test]
    fn add_same_entry_twice() {
        let mut failed = FailedEntries {
            entries: vec![new_store_entry("213", "gog")],
        };

        assert_eq!(add(new_store_entry("123", "gog"), &mut failed), true);
        assert_eq!(failed.entries.len(), 2);
        assert_eq!(add(new_store_entry("123", "gog"), &mut failed), false);
        assert_eq!(failed.entries.len(), 2);
    }

    #[test]
    fn add_same_id_different_store() {
        let mut failed = FailedEntries {
            entries: vec![new_store_entry("213", "gog")],
        };

        assert_eq!(add(new_store_entry("123", "gog"), &mut failed), true);
        assert_eq!(failed.entries.len(), 2);
        assert_eq!(add(new_store_entry("123", "steam"), &mut failed), true);
        assert_eq!(failed.entries.len(), 3);
    }

    #[test]
    fn remove_from_empty_library() {
        let mut failed = FailedEntries { entries: vec![] };

        assert_eq!(remove(&new_store_entry("123", "gog"), &mut failed), false);
        assert_eq!(failed.entries.len(), 0);
    }

    #[test]
    fn remove_from_non_empty_library_not_found() {
        let mut failed = FailedEntries {
            entries: vec![new_store_entry("213", "gog")],
        };

        assert_eq!(remove(&new_store_entry("123", "gog"), &mut failed), false);
        assert_eq!(failed.entries.len(), 1);
    }

    #[test]
    fn remove_from_library_found() {
        let mut failed = FailedEntries {
            entries: vec![new_store_entry("213", "gog"), new_store_entry("123", "gog")],
        };

        assert_eq!(remove(&new_store_entry("123", "gog"), &mut failed), true);
        assert_eq!(failed.entries.len(), 1);
    }

    #[test]
    fn remove_from_library_same_id_different_store_exists() {
        let mut failed = FailedEntries {
            entries: vec![new_store_entry("213", "gog"), new_store_entry("123", "gog")],
        };

        assert_eq!(remove(&new_store_entry("123", "steam"), &mut failed), false);
        assert_eq!(failed.entries.len(), 2);
    }
}
