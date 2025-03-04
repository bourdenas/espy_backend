use crate::{
    api::FirestoreApi,
    documents::{GameDigest, GameEntry, LibraryEntry, StoreEntry, Unresolved},
    resolver::ResolveApi,
    Status,
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tracing::{error, info_span, instrument, Instrument};

use super::firestore::{self, external_games, games};

pub struct LibraryManager {
    user_id: String,
}

impl LibraryManager {
    /// Creates a LibraryManager instance for a user.
    pub fn new(user_id: &str) -> Self {
        LibraryManager {
            user_id: String::from(user_id),
        }
    }

    #[instrument(level = "info", skip(self, firestore, resolver, store_entries))]
    pub async fn add_in_library(
        &self,
        firestore: Arc<FirestoreApi>,
        resolver: Arc<ResolveApi>,
        store_entries: Vec<StoreEntry>,
    ) -> Result<(), Status> {
        if store_entries.is_empty() {
            return Ok(());
        }

        let externals = external_games::batch_read(&firestore, store_entries).await?;

        let doc_ids =
            HashSet::<u64>::from_iter(externals.matches.iter().map(|m| m.external_game.igdb_id))
                .into_iter()
                .collect_vec();

        let result = games::batch_read(&firestore, &doc_ids).await?;
        let games = HashMap::<u64, GameEntry>::from_iter(
            result.documents.into_iter().map(|game| (game.id, game)),
        );
        let not_found_games = externals
            .matches
            .iter()
            .filter(|m| !games.contains_key(&m.external_game.igdb_id))
            .map(|m| m.clone())
            .collect_vec();

        // Resolve from IGDB games that were not found.
        if !not_found_games.is_empty() {
            let resolver = Arc::clone(&resolver);
            let firestore = Arc::clone(&firestore);
            let user_id = self.user_id.clone();
            tokio::spawn(
                async move {
                    igdb_resolve(firestore, resolver, user_id, not_found_games).await;
                }
                .instrument(info_span!("spawn_igdb_resolve")),
            );
        }

        let library_entries = externals
            .matches
            .iter()
            .filter(|m| games.contains_key(&m.external_game.igdb_id))
            .flat_map(|m| {
                let game_entry = games.get(&m.external_game.igdb_id).unwrap();
                LibraryEntry::new_with_expand(game_entry.clone(), m.store_entry.clone())
            })
            .collect_vec();

        if !library_entries.is_empty() {
            let game_ids = library_entries.iter().map(|e| e.id).collect_vec();
            firestore::library::add_entries(&firestore, &self.user_id, library_entries).await?;
            firestore::wishlist::remove_entries(&firestore, &self.user_id, &game_ids).await?;
        }

        // For games that were not found in ExternalGames generate candidates
        // by searching their titles in IGDB.
        if !externals.not_found.is_empty() {
            let firestore = Arc::clone(&firestore);
            let user_id = self.user_id.clone();
            let missing = externals.not_found.clone();
            tokio::spawn(
                async move {
                    search_candidates(firestore, resolver, user_id, missing).await;
                }
                .instrument(info_span!("spawn_search_candidates")),
            );
        }

        firestore::storefront::add_entries(
            &firestore,
            &self.user_id,
            externals
                .matches
                .into_iter()
                .map(|m| m.store_entry)
                .chain(externals.not_found)
                .collect_vec(),
        )
        .await?;

        Ok(())
    }

    #[instrument(
        level = "trace",
        skip(self, firestore, store_entry, game_entry)
        fields(
            store_game = %store_entry.title,
        ),
    )]
    pub async fn create_library_entry(
        &self,
        firestore: Arc<FirestoreApi>,
        store_entry: StoreEntry,
        game_entry: GameEntry,
    ) -> Result<(), Status> {
        firestore::unresolved::remove_entry(&firestore, &self.user_id, &store_entry).await?;

        let library_entries = LibraryEntry::new_with_expand(game_entry, store_entry);
        firestore::wishlist::remove_entries(
            &firestore,
            &self.user_id,
            &library_entries.iter().map(|e| e.id).collect_vec(),
        )
        .await?;
        firestore::library::add_entries(&firestore, &self.user_id, library_entries).await
    }

    /// Unmatch a `StoreEntry` from user's library.
    ///
    /// If `delete` is false, the StoreEntry is not deleted, but instead moved
    /// to failed matches.
    #[instrument(level = "trace", skip(self, firestore))]
    pub async fn unmatch_game(
        &self,
        firestore: Arc<FirestoreApi>,
        store_entry: StoreEntry,
        delete: bool,
    ) -> Result<(), Status> {
        firestore::library::remove_entry(&firestore, &self.user_id, &store_entry).await?;
        if delete {
            firestore::storefront::remove_entry(&firestore, &self.user_id, &store_entry).await
        } else {
            firestore::unresolved::add_unknown(&firestore, &self.user_id, vec![store_entry]).await
        }
    }

    #[instrument(
        level = "trace",
        skip(self, firestore, store_entry, game_entry)
        fields(
            store_game = %store_entry.title,
        ),
    )]
    pub async fn rematch_game(
        &self,
        firestore: Arc<FirestoreApi>,
        store_entry: StoreEntry,
        game_entry: GameEntry,
    ) -> Result<(), Status> {
        firestore::library::replace_entry(
            &firestore,
            &self.user_id,
            &store_entry,
            LibraryEntry::new_with_expand(game_entry, store_entry.clone()),
        )
        .await
    }

    #[instrument(
        level = "trace",
        skip(self, firestore, game_entry),
        fields(
            game_id = %game_entry.id,
        )
    )]
    pub async fn update_game(
        &self,
        firestore: Arc<FirestoreApi>,
        game_entry: GameEntry,
    ) -> Result<(), Status> {
        let game_digest = GameDigest::from(game_entry);
        match firestore::library::update_entry(&firestore, &self.user_id, game_digest.clone()).await
        {
            Ok(()) => Ok(()),
            Err(Status::NotFound(_)) => {
                firestore::wishlist::update_entry(&firestore, &self.user_id, game_digest).await
            }
            Err(e) => Err(e),
        }
    }

    #[instrument(level = "trace", skip(self, firestore))]
    pub async fn add_to_wishlist(
        &self,
        firestore: Arc<FirestoreApi>,
        library_entry: LibraryEntry,
    ) -> Result<(), Status> {
        firestore::wishlist::add_entry(&firestore, &self.user_id, library_entry).await
    }

    #[instrument(level = "trace", skip(self, firestore))]
    pub async fn remove_from_wishlist(
        &self,
        firestore: Arc<FirestoreApi>,
        game_id: u64,
    ) -> Result<(), Status> {
        firestore::wishlist::remove_entry(&firestore, &self.user_id, game_id).await
    }

    /// Remove all entries in user library from specified storefront.
    #[instrument(level = "trace", skip(self, firestore))]
    pub async fn remove_storefront(
        &self,
        firestore: Arc<FirestoreApi>,
        storefront_id: &str,
    ) -> Result<(), Status> {
        firestore::library::remove_storefront(&firestore, &self.user_id, storefront_id).await?;
        firestore::unresolved::remove_storefront(&firestore, &self.user_id, storefront_id).await?;
        firestore::storefront::remove_store(&firestore, &self.user_id, storefront_id).await
    }
}

async fn igdb_resolve(
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    user_id: String,
    externals: Vec<external_games::ExternalMatch>,
) {
    let mut library_entries = vec![];
    for m in externals {
        let id = m.external_game.igdb_id;
        let game_entry = match resolver.retrieve(id).await {
            Ok(mut game_entry) => match games::write(&firestore, &mut game_entry).await {
                Ok(()) => game_entry,
                Err(status) => {
                    error!("Failed to store GameEntry: {status}");
                    continue;
                }
            },
            Err(status) => {
                error!("Failed to retribe IGDB game: {status}");
                continue;
            }
        };

        library_entries.extend(LibraryEntry::new_with_expand(game_entry, m.store_entry));
    }

    let game_ids = library_entries.iter().map(|e| e.id).collect_vec();
    if let Err(e) = firestore::library::add_entries(&firestore, &user_id, library_entries).await {
        error!("{e}");
    }
    if let Err(e) = firestore::wishlist::remove_entries(&firestore, &user_id, &game_ids).await {
        error!("{e}");
    }
}

async fn search_candidates(
    firestore: Arc<FirestoreApi>,
    resolver: Arc<ResolveApi>,
    user_id: String,
    missing: Vec<StoreEntry>,
) {
    let mut unresolved = vec![];
    let mut unknown = vec![];
    for store_entry in missing {
        match resolver.search(store_entry.title.clone(), false).await {
            Ok(candidates) => {
                if !candidates.is_empty() {
                    unresolved.push(Unresolved {
                        store_entry,
                        candidates: candidates
                            .into_iter()
                            .map(|game_entry| GameDigest::from(game_entry))
                            .collect(),
                    });
                } else {
                    unknown.push(store_entry);
                }
            }
            Err(status) => {
                error!("{status}");
            }
        }
    }

    if let Err(status) =
        firestore::unresolved::add_unresolved(&firestore, &user_id, unresolved, unknown).await
    {
        error!("{status}");
    }
}
