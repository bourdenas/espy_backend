use crate::{
    api::FirestoreApi,
    documents::{GameDigest, Library, LibraryEntry, StoreEntry},
    Status,
};
use tracing::instrument;

use super::utils;

#[instrument(name = "library::read", level = "trace", skip(firestore, user_id))]
pub async fn read(firestore: &FirestoreApi, user_id: &str) -> Result<Library, Status> {
    utils::auth_read(firestore, user_id, GAMES, LIBRARY_DOC.to_owned()).await
}

#[instrument(
    name = "library::write",
    level = "trace",
    skip(firestore, user_id, library)
)]
pub async fn write(
    firestore: &FirestoreApi,
    user_id: &str,
    mut library: Library,
) -> Result<(), Status> {
    library
        .entries
        .sort_by(|l, r| r.digest.release_date.cmp(&l.digest.release_date));
    utils::auth_write(firestore, user_id, GAMES, LIBRARY_DOC.to_owned(), &library).await
}

const GAMES: &str = "games";
const LIBRARY_DOC: &str = "library";

#[instrument(
    name = "library::add_entry",
    level = "trace",
    skip(firestore, user_id, digests)
)]
pub async fn add_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entry: StoreEntry,
    digests: Vec<GameDigest>,
) -> Result<(), Status> {
    let mut library = read(firestore, user_id).await?;

    for digest in digests {
        add(LibraryEntry::new(digest, store_entry.clone()), &mut library);
    }
    write(firestore, user_id, library).await
}

#[instrument(
    name = "library::add_entries",
    level = "trace",
    skip(firestore, user_id, library_entries)
)]
pub async fn add_entries(
    firestore: &FirestoreApi,
    user_id: &str,
    library_entries: Vec<LibraryEntry>,
) -> Result<(), Status> {
    let mut library = read(firestore, user_id).await?;

    for library_entry in library_entries {
        add(library_entry, &mut library);
    }
    write(firestore, user_id, library).await
}

#[instrument(
    name = "library::remove_entry",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn remove_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entry: &StoreEntry,
) -> Result<(), Status> {
    let mut library = read(firestore, user_id).await?;
    if remove(store_entry, &mut library) {
        write(firestore, user_id, library).await?;
    }
    Ok(())
}

#[instrument(
    name = "library::replace_entry",
    level = "trace",
    skip(firestore, user_id, library_entries)
)]
pub async fn replace_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    store_entry: &StoreEntry,
    library_entries: Vec<LibraryEntry>,
) -> Result<(), Status> {
    let mut library = read(firestore, user_id).await?;

    remove(store_entry, &mut library);
    for library_entry in library_entries {
        add(library_entry, &mut library);
    }

    write(firestore, user_id, library).await?;

    Ok(())
}

#[instrument(
    name = "library::update_entry",
    level = "trace",
    skip(firestore, user_id, game_digest)
)]
pub async fn update_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    game_digest: GameDigest,
) -> Result<(), Status> {
    let mut library = read(firestore, user_id).await?;

    match library.entries.iter_mut().find(|e| e.id == game_digest.id) {
        Some(existing_entry) => existing_entry.digest = game_digest,
        None => {
            return Err(Status::not_found("not in library"));
        }
    }

    write(firestore, user_id, library).await
}

#[instrument(
    name = "library::remove_storefront",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn remove_storefront(
    firestore: &FirestoreApi,
    user_id: &str,
    storefront_id: &str,
) -> Result<(), Status> {
    let mut library = read(firestore, user_id).await?;
    remove_storefront_entries(storefront_id, &mut library);
    write(firestore, user_id, library).await
}

/// Adds `LibraryEntry` in the library.
///
/// If an entry exists for the same game, it merges their store entries. Returns
/// true if the entry is added.
///
/// Expects that the LibraryEntry contains exactly one StoreEntry.
fn add(mut library_entry: LibraryEntry, library: &mut Library) -> bool {
    match library
        .entries
        .iter_mut()
        .find(|e| e.id == library_entry.id)
    {
        Some(existing_entry) => {
            if existing_entry
                .store_entries
                .iter()
                .all(|e| e != library_entry.store_entries.first().unwrap())
            {
                existing_entry
                    .store_entries
                    .push(library_entry.store_entries.remove(0));
            } else {
                return false;
            }
        }
        None => library.entries.push(library_entry),
    }

    true
}

/// Removes `StoreEntry` from the `Library`.
///
/// If the associated LibraryEntry in the library the whole LibraryEntry is also
/// removed. Returns true if input `StoreEntry` was found.
fn remove(store_entry: &StoreEntry, library: &mut Library) -> bool {
    let mut entry_found = false;
    library.entries.retain_mut(|e| {
        e.store_entries.retain(|se| {
            let retain = se.storefront_name != store_entry.storefront_name
                || se.id != store_entry.id
                || se.title != store_entry.title;
            if !retain {
                entry_found = true;
            }
            retain
        });

        return !e.store_entries.is_empty();
    });

    entry_found
}

/// Removes all entries in `Library` from a specified storefront.
fn remove_storefront_entries(storefront_id: &str, library: &mut Library) {
    library.entries.retain_mut(|library_entry| {
        library_entry
            .store_entries
            .retain(|store_entry| store_entry.storefront_name != storefront_id);

        return !library_entry.store_entries.is_empty();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(id: u64) -> GameDigest {
        GameDigest {
            id,
            ..Default::default()
        }
    }

    fn library_entry(id: u64) -> LibraryEntry {
        LibraryEntry {
            id,
            store_entries: vec![StoreEntry {
                id: "store_id_0".to_owned(),
                title: "Game Title".to_owned(),
                storefront_name: "gog".to_owned(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn library_entry_with_stores(id: u64, stores: Vec<(&str, &str)>) -> LibraryEntry {
        LibraryEntry {
            id,
            store_entries: stores
                .iter()
                .map(|(store_game_id, store)| StoreEntry {
                    id: store_game_id.to_string(),
                    title: "Game Title".to_owned(),
                    storefront_name: store.to_string(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn add_in_empty_library() {
        let mut library = Library { entries: vec![] };

        assert!(add(
            LibraryEntry::new(digest(7), StoreEntry::default()),
            &mut library
        ));
        assert_eq!(library.entries.len(), 1);
    }

    #[test]
    fn add_same_library_entry() {
        let mut library = Library {
            entries: vec![library_entry(7)],
        };

        assert!(add(
            LibraryEntry::new(digest(7), StoreEntry::default()),
            &mut library
        ));
        assert_eq!(library.entries.len(), 1);
        assert_eq!(library.entries[0].store_entries.len(), 2);
    }

    #[test]
    fn remove_non_existing_entry() {
        let mut library = Library { entries: vec![] };

        let library_entry = library_entry(7);
        assert_eq!(remove(&library_entry.store_entries[0], &mut library), false);
        assert_eq!(library.entries.len(), 0);
    }

    #[test]
    fn remove_entry_with_single_store() {
        let mut library = Library {
            entries: vec![
                library_entry_with_stores(7, vec![("store_id_0", "gog")]),
                library_entry_with_stores(3, vec![("store_id_1", "gog")]),
            ],
        };

        let library_entry = library_entry(7);
        assert!(remove(&library_entry.store_entries[0], &mut library,));
        assert_eq!(library.entries.len(), 1);
    }

    #[test]
    fn remove_entry_with_multiple_stores() {
        let mut library = Library {
            entries: vec![
                library_entry_with_stores(7, vec![("store_id_0", "gog"), ("steam id", "steam")]),
                library_entry_with_stores(3, vec![("store_id_1", "gog")]),
            ],
        };

        let library_entry = library_entry(7);
        assert!(remove(&library_entry.store_entries[0], &mut library,));
        assert_eq!(library.entries.len(), 2);
        assert_eq!(library.entries[0].store_entries.len(), 1);
    }

    #[test]
    fn remove_entry_with_many_library_entries() {
        let mut library = Library {
            entries: vec![
                library_entry_with_stores(7, vec![("store_id_0", "gog"), ("steam id", "steam")]),
                library_entry_with_stores(12, vec![("store_id_0", "gog")]),
                library_entry_with_stores(15, vec![("store_id_0", "gog")]),
                library_entry_with_stores(24, vec![("store_id_1", "gog")]),
            ],
        };

        let library_entry = library_entry(7);
        assert!(remove(&library_entry.store_entries[0], &mut library,));
        assert_eq!(library.entries.len(), 2);
        assert_eq!(library.entries[0].store_entries.len(), 1);
    }

    #[test]
    fn remove_found_library_entry_but_not_store_entry() {
        let mut library = Library {
            entries: vec![library_entry(7), library_entry(3)],
        };

        assert_eq!(
            remove(
                &StoreEntry {
                    id: "some id".to_owned(),
                    title: "Game Title".to_owned(),
                    storefront_name: "steam".to_owned(),
                    ..Default::default()
                },
                &mut library,
            ),
            false
        );
        assert_eq!(library.entries.len(), 2);
        assert_eq!(library.entries[0].store_entries.len(), 1);
    }

    #[test]
    fn remove_all_storefront_entries() {
        let mut library = Library {
            entries: vec![
                library_entry_with_stores(7, vec![("gog_123", "gog")]),
                library_entry_with_stores(3, vec![("gog_213", "gog")]),
            ],
        };

        remove_storefront_entries("gog", &mut library);
        assert_eq!(library.entries.len(), 0);
    }

    #[test]
    fn remove_all_storefront_entries_store_does_not_exist() {
        let mut library = Library {
            entries: vec![
                library_entry_with_stores(7, vec![("gog_123", "gog")]),
                library_entry_with_stores(3, vec![("gog_213", "gog")]),
            ],
        };

        remove_storefront_entries("steam", &mut library);
        assert_eq!(library.entries.len(), 2);
    }

    #[test]
    fn remove_all_storefront_entries_does_not_affect_other_stores() {
        let mut library = Library {
            entries: vec![
                library_entry_with_stores(7, vec![("gog_123", "gog")]),
                library_entry_with_stores(2, vec![("steam_123", "steam")]),
                library_entry_with_stores(3, vec![("gog_213", "gog")]),
                library_entry_with_stores(5, vec![("steam_231", "steam")]),
            ],
        };

        remove_storefront_entries("gog", &mut library);
        assert_eq!(library.entries.len(), 2);
    }

    #[test]
    fn remove_all_storefront_entries_maintain_entry_with_other_store() {
        let mut library = Library {
            entries: vec![
                library_entry_with_stores(7, vec![("gog_123", "gog"), ("steam_321", "steam")]),
                library_entry_with_stores(2, vec![("steam_123", "steam")]),
                library_entry_with_stores(3, vec![("gog_213", "gog")]),
                library_entry_with_stores(5, vec![("steam_231", "steam")]),
            ],
        };

        remove_storefront_entries("gog", &mut library);
        assert_eq!(library.entries.len(), 3);
    }
}
