use crate::{
    api::{FirestoreApi, IgdbApi},
    documents::{GameEntry, LibraryEntry, StoreEntry},
    games::{ReconReport, Reconciler, Resolver, SteamDataApi},
    Status,
};
use std::sync::{Arc, Mutex};
use tracing::instrument;

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

    /// Reconciles store entries from the unmatched collection in Firestore.
    #[instrument(
        level = "trace",
        skip(self, igdb, steam),
        fields(user_id = %self.user_id),
    )]
    pub async fn recon_unmatched_collection(
        &self,
        igdb: Arc<IgdbApi>,
        steam: Arc<SteamDataApi>,
    ) -> Result<ReconReport, Status> {
        let unmatched_entries =
            firestore::unmatched::list(&self.firestore.lock().unwrap(), &self.user_id)?;
        self.recon_store_entries(unmatched_entries, igdb, steam)
            .await
    }

    /// Reconciles `store_entries` and adds them in the library.
    #[instrument(
        level = "trace",
        skip(self, store_entries, igdb, steam),
        fields(
            user_id = %self.user_id,
            entries_num = %store_entries.len()
        ),
    )]
    pub async fn recon_store_entries(
        &self,
        store_entries: Vec<StoreEntry>,
        igdb: Arc<IgdbApi>,
        steam: Arc<SteamDataApi>,
    ) -> Result<ReconReport, Status> {
        let mut report = ReconReport {
            lines: vec![format!(
                "Attempted to match {} new entries.",
                store_entries.len()
            )],
        };

        for store_entry in store_entries {
            let igdb = Arc::clone(&igdb);
            let steam = Arc::clone(&steam);
            let game_entry = Reconciler::recon(&igdb, &store_entry, false).await?;

            match game_entry {
                Some(game_entry) => {
                    report.lines.push(format!(
                        "  matched '{}' ({}) with {}",
                        &store_entry.title, &store_entry.storefront_name, &game_entry.name,
                    ));
                    self.match_game(store_entry, game_entry, igdb, steam, MatchType::BaseGame)
                        .await?
                }
                None => {
                    report.lines.push(format!(
                        "  failed to match {} ({})",
                        &store_entry.title, &store_entry.storefront_name,
                    ));
                    firestore::failed::add_entry(
                        &self.firestore.lock().unwrap(),
                        &self.user_id,
                        store_entry,
                    )?
                }
            }
        }

        Ok(report)
    }

    /// Match a `StoreEntry` with a specified `GameEntry` and saving it in the
    /// library.
    #[instrument(level = "trace", skip(self, igdb, steam))]
    pub async fn match_game(
        &self,
        store_entry: StoreEntry,
        game_entry: GameEntry,
        igdb: Arc<IgdbApi>,
        steam: Arc<SteamDataApi>,
        match_type: MatchType,
    ) -> Result<(), Status> {
        let owned_game_id = game_entry.id;
        let game_id = match (match_type, game_entry.parent) {
            (MatchType::BaseGame, Some(parent_id)) => parent_id,
            _ => game_entry.id,
        };

        let game_entry =
            match Resolver::retrieve(game_id, igdb, steam, Arc::clone(&self.firestore)).await? {
                Some(game_entry) => game_entry,
                None => {
                    return Err(Status::not_found(format!(
                        "Could not find game with id={game_id}"
                    )))
                }
            };

        let firestore = &self.firestore.lock().unwrap();
        firestore::unmatched::delete(firestore, &self.user_id, &store_entry)?;
        firestore::failed::remove_entry(firestore, &self.user_id, &store_entry)?;
        firestore::wishlist::remove_entry(firestore, &self.user_id, game_entry.id)?;
        firestore::library::add_entry(
            &firestore,
            &self.user_id,
            store_entry,
            owned_game_id,
            game_entry,
        )
    }

    /// Unmatch a `StoreEntry` from user's library. The StoreEntry is not
    /// deleted. Instead it is moved into the failed matches.
    #[instrument(level = "trace", skip(self, library_entry))]
    pub async fn unmatch_game(
        &self,
        store_entry: StoreEntry,
        library_entry: &LibraryEntry,
        delete: bool,
    ) -> Result<(), Status> {
        let firestore = &self.firestore.lock().unwrap();
        firestore::library::remove_entry(firestore, &self.user_id, &store_entry, library_entry)?;
        match delete {
            false => firestore::failed::add_entry(firestore, &self.user_id, store_entry),
            true => firestore::storefront::remove(firestore, &self.user_id, &store_entry),
        }
    }

    #[instrument(level = "trace", skip(self, igdb, steam))]
    pub async fn rematch_game(
        &self,
        store_entry: StoreEntry,
        game_entry: GameEntry,
        existing_library_entry: &LibraryEntry,
        igdb: Arc<IgdbApi>,
        steam: Arc<SteamDataApi>,
        match_type: MatchType,
    ) -> Result<(), Status> {
        let game_entry =
            match Resolver::retrieve(game_entry.id, igdb, steam, Arc::clone(&self.firestore))
                .await?
            {
                Some(game_entry) => game_entry,
                None => {
                    return Err(Status::not_found(format!(
                        "Could not find game with id={}",
                        game_entry.id
                    )))
                }
            };

        let firestore = &self.firestore.lock().unwrap();
        firestore::library::remove_entry(
            firestore,
            &self.user_id,
            &store_entry,
            existing_library_entry,
        )?;
        firestore::library::add_entry(
            firestore,
            &self.user_id,
            store_entry,
            game_entry.id, // TODO: This is probably incorrect.
            game_entry,
        )
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
}

#[derive(Debug)]
pub enum MatchType {
    Exact,
    BaseGame,
}
