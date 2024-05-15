use crate::{
    api::{FirestoreApi, IgdbApi},
    documents::{GameDigest, GameEntry, LibraryEntry, StoreEntry},
    games::Reconciler,
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
        println!("batching {} store entries", store_entries.len());
        let results = external_games::batch_read(&firestore, store_entries).await?;

        let doc_ids = HashSet::<u64>::from_iter(
            results
                .iter()
                .filter(|r| r.external_game.is_some())
                .map(|r| r.external_game.as_ref().unwrap().igdb_id),
        )
        .into_iter()
        .collect_vec();

        let (games, _not_found_games) = games::batch_read(&firestore, &doc_ids).await?;
        let games =
            HashMap::<u64, GameEntry>::from_iter(games.into_iter().map(|game| (game.id, game)));
        let not_found_games = results
            .iter()
            .filter(|r| r.external_game.is_some())
            .filter(|r| !games.contains_key(&r.external_game.as_ref().unwrap().igdb_id))
            .map(|r| r.clone())
            .collect_vec();

        println!("found {} games", games.len());
        println!("did not find {} games", not_found_games.len());

        let firestore_clone = Arc::clone(&firestore);
        let user_id = self.user_id.clone();
        if !not_found_games.is_empty() {
            tokio::spawn(
                async move {
                    let mut library_entries = vec![];
                    for result in not_found_games {
                        let id = result.external_game.as_ref().unwrap().igdb_id;
                        println!("Resolving missing '{}' ({id})", &result.store_entry.title);
                        let igdb_game = match igdb.get(id).await {
                            Ok(game) => game,
                            Err(status) => {
                                error!("Failed to retrieve IGDB game: {status}");
                                continue;
                            }
                        };
                        library_entries.extend(
                            match igdb.resolve(Arc::clone(&firestore_clone), igdb_game).await {
                                Ok(game) => LibraryEntry::new_with_expand(game, result.store_entry),
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

        let library_entries = results
            .iter()
            .filter(|r| r.external_game.is_some())
            .filter(|r| games.contains_key(&r.external_game.as_ref().unwrap().igdb_id))
            .flat_map(|r| {
                let game_entry = games
                    .get(&r.external_game.as_ref().unwrap().igdb_id)
                    .unwrap();
                LibraryEntry::new_with_expand(game_entry.clone(), r.store_entry.clone())
            })
            .collect_vec();

        let unmatched_store_entries = results
            .iter()
            .filter(|r| {
                r.external_game.is_none()
                    || !games.contains_key(&r.external_game.as_ref().unwrap().igdb_id)
            })
            .map(|r| r.store_entry.clone())
            .collect_vec();

        if !library_entries.is_empty() {
            let game_ids = library_entries.iter().map(|e| e.id).collect_vec();
            firestore::library::add_entries(&firestore, &self.user_id, library_entries).await?;
            firestore::wishlist::remove_entries(&firestore, &self.user_id, &game_ids).await?;
        }
        if !unmatched_store_entries.is_empty() {
            firestore::failed::add_entries(&firestore, &self.user_id, unmatched_store_entries)
                .await?;
        }
        if !results.is_empty() {
            firestore::storefront::add_entries(
                &firestore,
                &self.user_id,
                results.into_iter().map(|r| r.store_entry).collect_vec(),
            )
            .await?;
        }

        Ok(())
    }

    #[instrument(
        level = "trace",
        skip(self, firestore, igdb, store_entry),
        fields(
            title = %store_entry.title,
        )
    )]
    async fn match_entry(
        &self,
        firestore: Arc<FirestoreApi>,
        igdb: Arc<IgdbApi>,
        store_entry: StoreEntry,
    ) -> Result<(Vec<GameDigest>, StoreEntry), Status> {
        Ok((
            Reconciler::get_digest_by_store_entry(Arc::clone(&firestore), &igdb, &store_entry)
                .await?,
            store_entry,
        ))
    }

    #[instrument(level = "trace", skip(self, firestore, igdb))]
    pub async fn get_digest(
        &self,
        firestore: Arc<FirestoreApi>,
        igdb: Arc<IgdbApi>,
        game_id: u64,
    ) -> Result<Vec<GameDigest>, Status> {
        Reconciler::get_digest(Arc::clone(&firestore), &igdb, game_id).await
    }

    #[instrument(
        level = "trace",
        skip(self, firestore, store_entry, digests)
        fields(
            store_game = %store_entry.title,
        ),
    )]
    pub async fn create_library_entry(
        &self,
        firestore: Arc<FirestoreApi>,
        store_entry: StoreEntry,
        digests: Vec<GameDigest>,
    ) -> Result<(), Status> {
        firestore::failed::remove_entry(&firestore, &self.user_id, &store_entry).await?;
        for digest in &digests {
            firestore::wishlist::remove_entry(&firestore, &self.user_id, digest.id).await?;
        }
        firestore::library::add_entry(&firestore, &self.user_id, store_entry, digests).await
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
        match delete {
            false => firestore::failed::add_entry(&firestore, &self.user_id, store_entry).await,
            true => {
                firestore::storefront::remove_entry(&firestore, &self.user_id, &store_entry).await
            }
        }
    }

    #[instrument(
        level = "trace",
        skip(self, firestore, igdb, store_entry)
        fields(
            store_game = %store_entry.title,
        ),
    )]
    pub async fn rematch_game(
        &self,
        firestore: Arc<FirestoreApi>,
        igdb: Arc<IgdbApi>,
        store_entry: StoreEntry,
        game_id: u64,
    ) -> Result<(), Status> {
        let digests = self
            .get_digest(Arc::clone(&firestore), igdb, game_id)
            .await?;

        firestore::library::remove_entry(&firestore, &self.user_id, &store_entry).await?;
        firestore::library::add_entry(&firestore, &self.user_id, store_entry, digests).await
    }

    #[instrument(level = "trace", skip(self, firestore, igdb))]
    pub async fn update_game(
        &self,
        firestore: Arc<FirestoreApi>,
        igdb: Arc<IgdbApi>,
        game_id: u64,
    ) -> Result<(), Status> {
        let digests = self
            .get_digest(Arc::clone(&firestore), igdb, game_id)
            .await?;
        let result =
            firestore::library::update_entry(&firestore, &self.user_id, game_id, digests.clone())
                .await;
        match result {
            Ok(()) => Ok(()),
            Err(Status::NotFound(_)) => {
                firestore::wishlist::update_entry(&firestore, &self.user_id, game_id, digests).await
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
        firestore::failed::remove_storefront(&firestore, &self.user_id, storefront_id).await?;
        firestore::storefront::remove_store(&firestore, &self.user_id, storefront_id).await
    }
}
