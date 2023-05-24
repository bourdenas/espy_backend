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
                None => match_by_title(Arc::clone(&firestore), igdb, &store_entry.title).await?,
            };

        match game_entry {
            Some(game_entry) => Reconciler::expand_bundle(firestore, igdb, game_entry).await,
            None => Ok(vec![]),
        }
    }

    /// Returns the GameEntry corresponding to `game_id`.
    ///
    /// The GameEntry returned is not fully resolved but contains all fields
    /// necessary to build a GameDigest.
    ///
    /// It may return multiple matched games in the case of bundles or game
    /// versions that include expansions, DLCs, etc.
    #[instrument(level = "trace", skip(firestore, igdb))]
    pub async fn recon_by_id(
        firestore: Arc<Mutex<FirestoreApi>>,
        igdb: &IgdbApi,
        game_id: u64,
    ) -> Result<Vec<GameEntry>, Status> {
        let game_entry = {
            let firestore = &firestore.lock().unwrap();
            firestore::games::read(firestore, game_id)
        };
        let game_entry = match game_entry {
            Ok(game_entry) => Some(game_entry),
            Err(_) => match igdb.get(game_id).await {
                Ok(igdb_game) => match igdb.get_digest(Arc::clone(&firestore), &igdb_game).await {
                    Ok(game_entry) => Some(game_entry),
                    Err(_) => None,
                },
                Err(_) => None,
            },
        };

        match game_entry {
            Some(game_entry) => Reconciler::expand_bundle(firestore, igdb, game_entry).await,
            None => Ok(vec![]),
        }
    }

    async fn retrieve(
        firestore: Arc<Mutex<FirestoreApi>>,
        igdb: &IgdbApi,
        game_id: u64,
    ) -> Result<GameEntry, Status> {
        let game_entry = {
            let firestore = &firestore.lock().unwrap();
            firestore::games::read(firestore, game_id)
        };
        info!("    Fetching {game_id}");
        match game_entry {
            Ok(game_entry) => Ok(game_entry),
            Err(_) => match igdb.get(game_id).await {
                Ok(igdb_game) => igdb.get_digest(firestore, &igdb_game).await,
                Err(e) => Err(e),
            },
        }
    }

    /// If the GameEntry is a bundle it returns all GameEntries included in it.
    pub async fn expand_bundle(
        firestore: Arc<Mutex<FirestoreApi>>,
        igdb: &IgdbApi,
        game_entry: GameEntry,
    ) -> Result<Vec<GameEntry>, Status> {
        match game_entry.category {
            GameCategory::Bundle | GameCategory::Version => {
                let igdb_games = igdb.expand_bundle(game_entry.id).await?;
                info!(
                    "  Expanded '{}' to {} games",
                    game_entry.name,
                    igdb_games.len()
                );
                let mut games = vec![game_entry];
                if igdb_games.is_empty() {
                    if let Some(parent) = &games.first().unwrap().parent {
                        games.push(
                            Reconciler::retrieve(Arc::clone(&firestore), igdb, parent.id).await?,
                        );
                    }
                }
                for game in igdb_games {
                    games.push(Reconciler::retrieve(Arc::clone(&firestore), igdb, game.id).await?);
                }
                Ok(games)
            }
            _ => Ok(vec![game_entry]),
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
                    match igdb.get_digest(firestore, &igdb_game).await {
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
                Err(Status::NotFound(_)) => match igdb.get_digest(firestore, &igdb_game).await {
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
