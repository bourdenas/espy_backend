use crate::{
    api::{FirestoreApi, IgdbApi},
    documents::{GameDigest, LibraryEntry, StoreEntry},
    games::{ReconReport, Reconciler},
    Status,
};
use std::{sync::Arc, time::SystemTime};
use tracing::{error, instrument, trace_span, Instrument};

use super::firestore;

pub struct LibraryManager {
    user_id: String,
    firestore: Arc<FirestoreApi>,
}

impl LibraryManager {
    /// Creates a LibraryManager instance for a user.
    pub fn new(user_id: &str, firestore: Arc<FirestoreApi>) -> Self {
        LibraryManager {
            user_id: String::from(user_id),
            firestore,
        }
    }

    /// Reconciles `store_entries` and adds them in the library.
    #[instrument(
        level = "trace",
        skip(self, store_entries, igdb),
        fields(
            entries_num = %store_entries.len()
        ),
    )]
    pub async fn recon_store_entries(
        &self,
        store_entries: Vec<StoreEntry>,
        igdb: Arc<IgdbApi>,
    ) -> Result<ReconReport, Status> {
        let mut resolved_entries = vec![];
        let mut report = ReconReport {
            lines: vec![format!(
                "Attempted to match {} new entries.",
                store_entries.len()
            )],
        };

        // TODO: Errors will considered failed entry as resolved. Need to filter
        // entries with error (beyond failed to match, which is handled
        // correctly) and retry them.
        let mut last_updated = SystemTime::now();
        for store_entry in store_entries {
            let igdb = Arc::clone(&igdb);

            match self.match_entry(igdb, store_entry.clone()).await {
                Ok(result) => resolved_entries.push(result),
                Err(e) => {
                    // TODO: This needs to move somewhere else.
                    // e.g. inside the upload_entries.
                    firestore::failed::add_entry(&self.firestore, &self.user_id, store_entry)
                        .await?;
                    report.lines.push(e.to_string())
                }
            }

            // // Batch user library updates because writes on larges libraries
            // // become costly.
            // let time_passed = SystemTime::now().duration_since(last_updated).unwrap();
            // if time_passed.as_secs() > 3 {
            //     last_updated = SystemTime::now();
            //     // let entries = resolved_entries.drain(..).collect();
            //     let firestore = Arc::clone(&self.firestore);
            //     let user_id = self.user_id.clone();
            //     tokio::spawn(
            //         async move {
            //             if let Err(e) = Self::upload_entries(firestore, &user_id, entries).await {
            //                 error!("{e}");
            //             }
            //         }
            //         .instrument(trace_span!("spawn_library_update")),
            //     );
            // }
        }

        // Self::upload_entries(Arc::clone(&self.firestore), &self.user_id, resolved_entries).await?;

        Ok(report)
    }

    #[instrument(level = "trace", skip(firestore, user_id, entries))]
    async fn upload_entries(
        firestore: Arc<FirestoreApi>,
        user_id: &str,
        entries: Vec<(Vec<GameDigest>, StoreEntry)>,
    ) -> Result<(), Status> {
        let store_entries = entries
            .iter()
            .map(|(_, store_entry)| store_entry.clone())
            .collect();

        // Adds all resolved entries in the library.
        firestore::wishlist::remove_entries(
            &firestore,
            user_id,
            &entries
                .iter()
                .map(|(digests, _)| digests.iter().map(|digest| digest.id))
                .flatten()
                .collect::<Vec<_>>(),
        )
        .await?;
        firestore::library::add_entries(&firestore, user_id, entries).await?;
        firestore::storefront::add_entries(&firestore, user_id, store_entries).await
    }

    #[instrument(
        level = "trace",
        skip(self, igdb, store_entry),
        fields(
            title = %store_entry.title,
        )
    )]
    async fn match_entry(
        &self,
        igdb: Arc<IgdbApi>,
        store_entry: StoreEntry,
    ) -> Result<(Vec<GameDigest>, StoreEntry), Status> {
        Ok((
            Reconciler::get_digest_by_store_entry(Arc::clone(&self.firestore), &igdb, &store_entry)
                .await?,
            store_entry,
        ))
    }

    #[instrument(level = "trace", skip(self, igdb))]
    pub async fn get_digest(
        &self,
        igdb: Arc<IgdbApi>,
        game_id: u64,
    ) -> Result<Vec<GameDigest>, Status> {
        Reconciler::get_digest(Arc::clone(&self.firestore), &igdb, game_id).await
    }

    #[instrument(
        level = "trace",
        skip(self, store_entry, digests)
        fields(
            store_game = %store_entry.title,
        ),
    )]
    pub async fn create_library_entry(
        &self,
        store_entry: StoreEntry,
        digests: Vec<GameDigest>,
    ) -> Result<(), Status> {
        firestore::failed::remove_entry(&self.firestore, &self.user_id, &store_entry).await?;
        for digest in &digests {
            firestore::wishlist::remove_entry(&self.firestore, &self.user_id, digest.id).await?;
        }
        firestore::library::add_entry(&self.firestore, &self.user_id, store_entry, digests).await
    }

    /// Unmatch a `StoreEntry` from user's library.
    ///
    /// If `delete` is false, the StoreEntry is not deleted, but instead moved
    /// to failed matches.
    #[instrument(level = "trace", skip(self))]
    pub async fn unmatch_game(&self, store_entry: StoreEntry, delete: bool) -> Result<(), Status> {
        firestore::library::remove_entry(&self.firestore, &self.user_id, &store_entry).await?;
        match delete {
            false => {
                firestore::failed::add_entry(&self.firestore, &self.user_id, store_entry).await
            }
            true => {
                firestore::storefront::remove(&self.firestore, &self.user_id, &store_entry).await
            }
        }
    }

    #[instrument(
        level = "trace",
        skip(self, igdb, store_entry)
        fields(
            store_game = %store_entry.title,
        ),
    )]
    pub async fn rematch_game(
        &self,
        igdb: Arc<IgdbApi>,
        store_entry: StoreEntry,
        game_id: u64,
    ) -> Result<(), Status> {
        let digests = self.get_digest(igdb, game_id).await?;

        firestore::library::remove_entry(&self.firestore, &self.user_id, &store_entry).await?;
        firestore::library::add_entry(&self.firestore, &self.user_id, store_entry, digests).await
    }

    #[instrument(level = "trace", skip(self, igdb))]
    pub async fn update_game(&self, igdb: Arc<IgdbApi>, game_id: u64) -> Result<(), Status> {
        let digests = self.get_digest(igdb, game_id).await?;
        let result = firestore::library::update_entry(
            &self.firestore,
            &self.user_id,
            game_id,
            digests.clone(),
        )
        .await;
        match result {
            Ok(()) => Ok(()),
            Err(Status::NotFound(_)) => {
                firestore::wishlist::update_entry(&self.firestore, &self.user_id, game_id, digests)
                    .await
            }
            Err(e) => Err(e),
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn add_to_wishlist(&self, library_entry: LibraryEntry) -> Result<(), Status> {
        firestore::wishlist::add_entry(&self.firestore, &self.user_id, library_entry).await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn remove_from_wishlist(&self, game_id: u64) -> Result<(), Status> {
        firestore::wishlist::remove_entry(&self.firestore, &self.user_id, game_id).await
    }

    /// Remove all entries in user library from specified storefront.
    #[instrument(level = "trace", skip(self))]
    pub async fn remove_storefront(&self, storefront_id: &str) -> Result<(), Status> {
        firestore::library::remove_storefront(&self.firestore, &self.user_id, storefront_id)
            .await?;
        firestore::failed::remove_storefront(&self.firestore, &self.user_id, storefront_id).await?;
        firestore::storefront::delete(&self.firestore, &self.user_id, storefront_id).await
    }
}
