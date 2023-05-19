use std::sync::{Arc, Mutex};

use crate::{
    api::{FirestoreApi, IgdbApi},
    documents::{GameCategory, GameEntry, StoreEntry},
    library::firestore,
    Status,
};
use tracing::{info, instrument};

pub struct Reconciler;

impl Reconciler {
    /// Reconcile a `StoreEntry` with IGDB games.
    ///
    /// It initially tries to use the external game table for finding the
    /// corresponding entry. If that fails it performs a search by title and
    /// matches with the best candidate.
    ///
    /// It may return multiple matched games in the case of bundles or game
    /// versions that include expansions, DLCs, etc.
    #[instrument(level = "trace", skip(firestore, igdb, store_entry))]
    pub async fn recon(
        firestore: Arc<Mutex<FirestoreApi>>,
        igdb: &IgdbApi,
        store_entry: &StoreEntry,
    ) -> Result<Vec<GameEntry>, Status> {
        let game_entry =
            match match_by_external_id(Arc::clone(&firestore), igdb, store_entry).await? {
                Some(game_entry) => Some(game_entry),
                None => match_by_title(firestore, igdb, &store_entry.title).await?,
            };

        match game_entry {
            Some(game_entry) => match game_entry.category {
                GameCategory::Bundle | GameCategory::Version => {
                    let igdb_games = igdb.expand_bundle(game_entry.id).await?;
                    let mut games = vec![game_entry];
                    for game in igdb_games {
                        games.push(igdb.get_digest(&game).await?);
                    }
                    Ok(games)
                }
                _ => Ok(vec![game_entry]),
            },
            None => Ok(vec![]),
        }
    }
}

/// Returns a `GameEntry` from IGDB matching the external storefront id in
/// `store_entry`.
#[instrument(level = "trace", skip(firestore, igdb))]
async fn match_by_external_id(
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: &IgdbApi,
    store_entry: &StoreEntry,
) -> Result<Option<GameEntry>, Status> {
    info!("Matching external '{}'", &store_entry.title);

    match store_entry.id.is_empty() {
        false => {
            let external_game = {
                let firestore = &firestore.lock().unwrap();
                match firestore::external_games::read(
                    firestore,
                    &store_entry.storefront_name,
                    &store_entry.id,
                ) {
                    Ok(external_game) => external_game,
                    Err(Status::NotFound(_)) => return Ok(None),
                    Err(e) => return Err(e),
                }
            };
            let game_entry = {
                let firestore = &firestore.lock().unwrap();
                firestore::games::read(firestore, external_game.igdb_id)
            };
            match game_entry {
                Ok(game_entry) => Ok(Some(game_entry)),
                Err(Status::NotFound(_)) => {
                    let igdb_game = match igdb.get(external_game.igdb_id).await {
                        Ok(game) => game,
                        Err(Status::NotFound(_)) => return Ok(None),
                        Err(e) => return Err(e),
                    };
                    match igdb.get_digest(&igdb_game).await {
                        Ok(game) => Ok(Some(game)),
                        Err(Status::NotFound(_)) => Ok(None),
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        }
        true => Ok(None),
    }
}

/// Returns a `GameEntry` from IGDB matching the `title`.
#[instrument(level = "trace", skip(firestore, igdb))]
async fn match_by_title(
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: &IgdbApi,
    title: &str,
) -> Result<Option<GameEntry>, Status> {
    info!("Searching by title '{}'", title);

    let candidates = igdb.search_by_title(title).await?;
    match candidates.into_iter().next() {
        Some(igdb_game) => {
            let game_entry = { firestore::games::read(&firestore.lock().unwrap(), igdb_game.id) };
            match game_entry {
                Ok(game_entry) => Ok(Some(game_entry)),
                Err(Status::NotFound(_)) => match igdb.get_digest(&igdb_game).await {
                    Ok(game) => Ok(Some(game)),
                    Err(Status::NotFound(_)) => Ok(None),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            }
        }
        None => Ok(None),
    }
}
