use crate::{
    api::{FirestoreApi, IgdbApi},
    documents::{GameDigest, GameEntry, LibraryEntry, StoreEntry},
    Status,
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tracing::{error, instrument, trace_span, Instrument};

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

    pub async fn batch_recon_store_entries(
        &self,
        firestore: Arc<FirestoreApi>,
        igdb: Arc<IgdbApi>,
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

        let (games, _not_found_games) = games::batch_read(&firestore, &doc_ids).await?;
        let games =
            HashMap::<u64, GameEntry>::from_iter(games.into_iter().map(|game| (game.id, game)));
        let not_found_games = externals
            .matches
            .iter()
            .filter(|m| !games.contains_key(&m.external_game.igdb_id))
            .map(|m| m.clone())
            .collect_vec();

        let firestore_clone = Arc::clone(&firestore);
        let user_id = self.user_id.clone();
        if !not_found_games.is_empty() {
            tokio::spawn(
                async move {
                    let mut library_entries = vec![];
                    for m in not_found_games {
                        let id = m.external_game.igdb_id;
                        let igdb_game = match igdb.get(id).await {
                            Ok(game) => game,
                            Err(status) => {
                                error!("Failed to retrieve IGDB game: {status}");
                                continue;
                            }
                        };
                        library_entries.extend(
                            match igdb.resolve(Arc::clone(&firestore_clone), igdb_game).await {
                                Ok(game) => LibraryEntry::new_with_expand(game, m.store_entry),
                                Err(status) => {
                                    error!("Failed to resolve IGDB game: {status}");
                                    continue;
                                }
                            },
                        );
                    }

                    let game_ids = library_entries.iter().map(|e| e.id).collect_vec();
                    if let Err(status) =
                        firestore::library::add_entries(&firestore_clone, &user_id, library_entries)
                            .await
                    {
                        error!("{status}");
                    }
                    if let Err(status) =
                        firestore::wishlist::remove_entries(&firestore_clone, &user_id, &game_ids)
                            .await
                    {
                        error!("{status}");
                    }
                }
                .instrument(trace_span!("spawn_resolve_missing")),
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

        // TODO: For missing generate candidates by searching by title and create a new doc with these.
        if !externals.missing.is_empty() {
            firestore::unresolved::add_unknown(
                &firestore,
                &self.user_id,
                externals.missing.clone(),
            )
            .await?;
        }

        firestore::storefront::add_entries(
            &firestore,
            &self.user_id,
            externals
                .matches
                .into_iter()
                .map(|m| m.store_entry)
                .chain(externals.missing)
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
