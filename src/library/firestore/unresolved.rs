use crate::{
    api::FirestoreApi,
    documents::{StoreEntry, UnresolvedEntries},
    Status,
};
use tracing::instrument;

#[instrument(
    name = "unresolved::add_unknown",
    level = "trace",
    skip(firestore, user_id, store_entries)
)]
pub async fn add_unknown(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entries: Vec<StoreEntry>,
) -> Result<(), Status> {
    let mut unresolved = read(firestore, user_id).await?;
    unresolved.unknown.extend(store_entries);
    write(firestore, user_id, &unresolved).await
}

#[instrument(
    name = "unresolved::remove_entry",
    level = "trace",
    skip(firestore, user_id, store_entry),
    fields(store_entry_id = %store_entry.id),
)]
pub async fn remove_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entry: &StoreEntry,
) -> Result<(), Status> {
    let mut unresolved = read(firestore, user_id).await?;
    if remove(store_entry, &mut unresolved) {
        write(firestore, user_id, &unresolved).await?;
    }
    Ok(())
}

#[instrument(
    name = "unresolved::remove_storefront",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn remove_storefront(
    firestore: &FirestoreApi,
    user_id: &str,
    storefront_name: &str,
) -> Result<(), Status> {
    let mut unresolved = read(firestore, user_id).await?;
    remove_storefront_entries(storefront_name, &mut unresolved);
    write(firestore, user_id, &unresolved).await
}

/// Remove `StoreEntry` from the unresolved entries.
///
/// Returns true if the `StoreEntry` was found and removed, false otherwise.
fn remove(store_entry: &StoreEntry, unresolved: &mut UnresolvedEntries) -> bool {
    let original_len = unresolved.need_approval.len();
    unresolved
        .need_approval
        .retain(|e| e.store_entry != *store_entry);

    if unresolved.need_approval.len() != original_len {
        return true;
    }

    let original_len = unresolved.unknown.len();
    unresolved.unknown.retain(|e| *e != *store_entry);
    unresolved.unknown.len() != original_len
}

/// Remove all unresolved store entries from specified storefront.
fn remove_storefront_entries(storefront_name: &str, unresolved: &mut UnresolvedEntries) {
    unresolved
        .need_approval
        .retain(|e| e.store_entry.storefront_name != storefront_name);

    unresolved
        .unknown
        .retain(|store_entry| store_entry.storefront_name != storefront_name);
}

#[instrument(name = "unresolved::read", level = "trace", skip(firestore, user_id))]
pub async fn read(firestore: &FirestoreApi, user_id: &str) -> Result<UnresolvedEntries, Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(GAMES)
        .parent(&parent_path)
        .obj()
        .one(UNRESOLVED_DOC)
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Ok(UnresolvedEntries::default()),
    }
}

#[instrument(
    name = "unresolved::write",
    level = "trace",
    skip(firestore, user_id, unresolved)
)]
pub async fn write(
    firestore: &FirestoreApi,
    user_id: &str,
    unresolved: &UnresolvedEntries,
) -> Result<(), Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    firestore
        .db()
        .fluent()
        .update()
        .in_col(GAMES)
        .document_id(UNRESOLVED_DOC)
        .parent(&parent_path)
        .object(unresolved)
        .execute()
        .await?;
    Ok(())
}

const USERS: &str = "users";
const GAMES: &str = "games";
const UNRESOLVED_DOC: &str = "unresolved";

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

    // #[test]
    // fn add_in_empty_library() {
    //     let mut unresolved = UnresolvedEntries::default();

    //     assert_eq!(add(new_store_entry("123", "gog"), &mut unresolved), true);
    //     assert_eq!(unresolved.entries.len(), 1);
    // }

    // #[test]
    // fn add_in_non_empty_library() {
    //     let mut unresolved = UnresolvedEntries {
    //         entries: vec![new_store_entry("213", "gog")],
    //     };

    //     assert_eq!(add(new_store_entry("123", "gog"), &mut unresolved), true);
    //     assert_eq!(unresolved.entries.len(), 2);
    // }

    // #[test]
    // fn add_same_entry_twice() {
    //     let mut unresolved = UnresolvedEntries {
    //         entries: vec![new_store_entry("213", "gog")],
    //     };

    //     assert_eq!(add(new_store_entry("123", "gog"), &mut unresolved), true);
    //     assert_eq!(unresolved.entries.len(), 2);
    //     assert_eq!(add(new_store_entry("123", "gog"), &mut unresolved), false);
    //     assert_eq!(unresolved.entries.len(), 2);
    // }

    // #[test]
    // fn add_same_id_different_store() {
    //     let mut unresolved = UnresolvedEntries {
    //         entries: vec![new_store_entry("213", "gog")],
    //     };

    //     assert_eq!(add(new_store_entry("123", "gog"), &mut unresolved), true);
    //     assert_eq!(unresolved.entries.len(), 2);
    //     assert_eq!(add(new_store_entry("123", "steam"), &mut unresolved), true);
    //     assert_eq!(unresolved.entries.len(), 3);
    // }

    #[test]
    fn remove_from_empty() {
        let mut unresolved = UnresolvedEntries::default();

        assert_eq!(
            remove(&new_store_entry("123", "gog"), &mut unresolved),
            false
        );
        assert_eq!(unresolved.need_approval.len(), 0);
        assert_eq!(unresolved.unknown.len(), 0);
    }

    #[test]
    fn remove_not_found() {
        let mut unresolved = UnresolvedEntries {
            need_approval: vec![],
            unknown: vec![new_store_entry("213", "gog")],
        };

        assert_eq!(
            remove(&new_store_entry("123", "gog"), &mut unresolved),
            false
        );
        assert_eq!(unresolved.need_approval.len(), 0);
        assert_eq!(unresolved.unknown.len(), 1);
    }

    #[test]
    fn remove_found() {
        let mut unresolved = UnresolvedEntries {
            need_approval: vec![],
            unknown: vec![new_store_entry("213", "gog"), new_store_entry("123", "gog")],
        };

        assert_eq!(
            remove(&new_store_entry("123", "gog"), &mut unresolved),
            true
        );
        assert_eq!(unresolved.need_approval.len(), 0);
        assert_eq!(unresolved.unknown.len(), 1);
    }

    #[test]
    fn remove_same_id_different_store_exists() {
        let mut unresolved = UnresolvedEntries {
            need_approval: vec![],
            unknown: vec![new_store_entry("213", "gog"), new_store_entry("123", "gog")],
        };

        assert_eq!(
            remove(&new_store_entry("123", "steam"), &mut unresolved),
            false
        );
        assert_eq!(unresolved.need_approval.len(), 0);
        assert_eq!(unresolved.unknown.len(), 2);
    }
}
