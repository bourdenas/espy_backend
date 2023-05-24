use crate::{
    api::{FirestoreApi, IgdbApi},
    documents::{GameEntry, LibraryEntry, StoreEntry},
    games::{ReconReport, Reconciler},
    Status,
};
use std::sync::{Arc, Mutex};
use tracing::{error, instrument, trace_span, Instrument};

use super::firestore;

pub struct LibraryManager {
    user_id: String,
    firestore: Arc<Mutex<FirestoreApi>>,
}

impl LibraryManager {
    /// Creates a LibraryManager instance for a user.
    pub fn new(user_id: &str, firestore: Arc<Mutex<FirestoreApi>>) -> Self {
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
        for store_entry in store_entries {
            let igdb = Arc::clone(&igdb);

            match self.match_entry(igdb, store_entry.clone()).await {
                Ok(result) => resolved_entries.push(result),
                Err(e) => {
                    // TODO: This needs to move somewhere else.
                    // e.g. inside the upload_entries.
                    firestore::failed::add_entry(
                        &self.firestore.lock().unwrap(),
                        &self.user_id,
                        store_entry,
                    )?;
                    report.lines.push(e.to_string())
                }
            }

            // Batch user library updates because writes on larges libraries
            // become costly.
            if resolved_entries.len() == 10 {
                let entries = resolved_entries.drain(..).collect();
                let firestore = Arc::clone(&self.firestore);
                let user_id = self.user_id.clone();
                tokio::spawn(
                    async move {
                        if let Err(e) = Self::upload_entries(firestore, &user_id, entries) {
                            error!("{e}");
                        }
                    }
                    .instrument(trace_span!("spawn_library_update")),
                );
            }
        }

        Self::upload_entries(Arc::clone(&self.firestore), &self.user_id, resolved_entries)?;

        Ok(report)
    }

    #[instrument(level = "trace", skip(firestore, user_id, entries))]
    fn upload_entries(
        firestore: Arc<Mutex<FirestoreApi>>,
        user_id: &str,
        entries: Vec<(Vec<GameEntry>, StoreEntry)>,
    ) -> Result<(), Status> {
        let store_entries = entries
            .iter()
            .map(|(_, store_entry)| store_entry.clone())
            .collect();

        // Adds all resolved entries in the library.
        // TODO: Should also remove entries from wishlist.
        let firestore = &firestore.lock().unwrap();
        firestore::library::add_entries(firestore, user_id, entries)?;
        firestore::storefront::add_entries(firestore, user_id, store_entries)
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
    ) -> Result<(Vec<GameEntry>, StoreEntry), Status> {
        Ok((
            Reconciler::recon(Arc::clone(&self.firestore), &igdb, &store_entry).await?,
            store_entry,
        ))
    }

    #[instrument(level = "trace", skip(self, igdb))]
    pub async fn get_game_entry(
        &self,
        igdb: Arc<IgdbApi>,
        game_id: u64,
    ) -> Result<Vec<GameEntry>, Status> {
        Reconciler::recon_by_id(Arc::clone(&self.firestore), &igdb, game_id).await
    }

    #[instrument(
        level = "trace",
        skip(self, store_entry, game_entries)
        fields(
            store_game = %store_entry.title,
        ),
    )]
    pub fn create_library_entry(
        &self,
        store_entry: StoreEntry,
        game_entries: Vec<GameEntry>,
    ) -> Result<(), Status> {
        let firestore = &self.firestore.lock().unwrap();
        firestore::failed::remove_entry(firestore, &self.user_id, &store_entry)?;
        for game_entry in &game_entries {
            firestore::wishlist::remove_entry(firestore, &self.user_id, game_entry.id)?;
        }
        firestore::library::add_entry(firestore, &self.user_id, store_entry, game_entries)
    }

    /// Unmatch a `StoreEntry` from user's library.
    ///
    /// If `delete` is false, the StoreEntry is not deleted, but instead moved
    /// to failed matches.
    #[instrument(level = "trace", skip(self))]
    pub async fn unmatch_game(&self, store_entry: StoreEntry, delete: bool) -> Result<(), Status> {
        let firestore = &self.firestore.lock().unwrap();
        firestore::library::remove_entry(firestore, &self.user_id, &store_entry)?;
        match delete {
            false => firestore::failed::add_entry(firestore, &self.user_id, store_entry),
            true => firestore::storefront::remove(firestore, &self.user_id, &store_entry),
        }
    }

    #[instrument(
        level = "trace",
        skip(self, igdb, store_entry, game_entry)
        fields(
            store_game = %store_entry.title,
            matched_game = %game_entry.name
        ),
    )]
    pub async fn rematch_game(
        &self,
        igdb: Arc<IgdbApi>,
        store_entry: StoreEntry,
        game_entry: GameEntry,
    ) -> Result<(), Status> {
        let game_entry = self.get_game_entry(igdb, game_entry.id).await?;

        let firestore = &self.firestore.lock().unwrap();
        firestore::library::remove_entry(firestore, &self.user_id, &store_entry)?;
        firestore::library::add_entry(firestore, &self.user_id, store_entry, game_entry)
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn add_to_wishlist(&self, library_entry: LibraryEntry) -> Result<(), Status> {
        firestore::wishlist::add_entry(
            &self.firestore.lock().unwrap(),
            &self.user_id,
            library_entry,
        )
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn remove_from_wishlist(&self, game_id: u64) -> Result<(), Status> {
        firestore::wishlist::remove_entry(&self.firestore.lock().unwrap(), &self.user_id, game_id)
    }

    /// Remove all entries in user library from specified storefront.
    #[instrument(level = "trace", skip(self))]
    pub async fn remove_storefront(&self, storefront_id: &str) -> Result<(), Status> {
        let firestore = &self.firestore.lock().unwrap();

        firestore::library::remove_storefront(firestore, &self.user_id, storefront_id)?;
        firestore::failed::remove_storefront(firestore, &self.user_id, storefront_id)?;
        firestore::storefront::delete(firestore, &self.user_id, storefront_id)
    }
}
