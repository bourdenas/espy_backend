use std::sync::{Arc, Mutex};

use crate::{
    api::{FirestoreApi, IgdbApi},
    documents::{GameCategory, GameDigest, StoreEntry},
    library::firestore,
    Status,
};
use async_recursion::async_recursion;
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
    pub async fn get_digest_by_store_entry(
        firestore: Arc<Mutex<FirestoreApi>>,
        igdb: &IgdbApi,
        store_entry: &StoreEntry,
    ) -> Result<Vec<GameDigest>, Status> {
        let game_entry =
            match match_by_external_id(Arc::clone(&firestore), igdb, store_entry).await? {
                Some(game_entry) => Some(game_entry),
                None => match_by_title(Arc::clone(&firestore), igdb, &store_entry.title).await?,
            };

        match game_entry {
            Some(game_entry) => Reconciler::expand(firestore, igdb, game_entry).await,
            None => Ok(vec![]),
        }
    }

    /// Returns the GameDigest corresponding to `game_id`.
    ///
    /// It may return multiple digests in the case of bundles and game versions
    /// that include expansions, DLCs, etc. or episodes that are part of series.
    #[instrument(level = "trace", skip(firestore, igdb))]
    pub async fn get_digest(
        firestore: Arc<Mutex<FirestoreApi>>,
        igdb: &IgdbApi,
        game_id: u64,
    ) -> Result<Vec<GameDigest>, Status> {
        let game_entry = {
            let firestore = &firestore.lock().unwrap();
            firestore::games::read(firestore, game_id)
        };
        let digest = match game_entry {
            Ok(game_entry) => Ok(GameDigest::from(game_entry)),
            Err(_) => match igdb.get(game_id).await {
                Ok(igdb_game) => match igdb.get_digest(Arc::clone(&firestore), &igdb_game).await {
                    Ok(digest) => Ok(digest),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            },
        };

        match digest {
            Ok(digest) => Reconciler::expand(firestore, igdb, digest).await,
            Err(e) => Err(e),
        }
    }

    /// If the game is a bundle it returns all games included in it.
    #[async_recursion]
    pub async fn expand(
        firestore: Arc<Mutex<FirestoreApi>>,
        igdb: &IgdbApi,
        digest: GameDigest,
    ) -> Result<Vec<GameDigest>, Status> {
        match digest.category {
            GameCategory::Bundle | GameCategory::Version => {
                let igdb_games = igdb.expand_bundle(digest.id).await?;

                let mut digests = vec![digest];
                if igdb_games.is_empty() {
                    if let Some(parent_id) = digests.first().unwrap().parent_id {
                        digests.extend(
                            Self::get_digest(Arc::clone(&firestore), igdb, parent_id).await?,
                        );
                    }
                }

                for game in igdb_games {
                    digests.extend(Self::get_digest(Arc::clone(&firestore), igdb, game.id).await?);
                }

                Ok(digests)
            }
            GameCategory::Episode => {
                let digests = match digest.parent_id {
                    Some(parent_id) => vec![
                        vec![digest],
                        Self::get_digest(firestore, igdb, parent_id).await?,
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                    None => vec![digest],
                };
                Ok(digests)
            }
            _ => Ok(vec![digest]),
        }
    }
}

/// Returns a `GameDigest` from IGDB matching the external storefront id in
/// `store_entry`.
#[instrument(level = "trace", skip(firestore, igdb))]
async fn match_by_external_id(
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: &IgdbApi,
    store_entry: &StoreEntry,
) -> Result<Option<GameDigest>, Status> {
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
                Ok(game_entry) => Ok(Some(GameDigest::from(game_entry))),
                Err(Status::NotFound(_)) => {
                    let igdb_game = match igdb.get(external_game.igdb_id).await {
                        Ok(igdb_game) => igdb_game,
                        Err(Status::NotFound(_)) => return Ok(None),
                        Err(e) => return Err(e),
                    };
                    match igdb.get_digest(firestore, &igdb_game).await {
                        Ok(digest) => Ok(Some(digest)),
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

/// Returns a `GameDigest` from IGDB matching the `title`.
#[instrument(level = "trace", skip(firestore, igdb))]
async fn match_by_title(
    firestore: Arc<Mutex<FirestoreApi>>,
    igdb: &IgdbApi,
    title: &str,
) -> Result<Option<GameDigest>, Status> {
    info!("Searching by title '{}'", title);

    let candidates = igdb.search_by_title(title).await?;
    match candidates.into_iter().next() {
        Some(igdb_game) => {
            let game_entry = { firestore::games::read(&firestore.lock().unwrap(), igdb_game.id) };
            match game_entry {
                Ok(game_entry) => Ok(Some(GameDigest::from(game_entry))),
                Err(Status::NotFound(_)) => match igdb.get_digest(firestore, &igdb_game).await {
                    Ok(digest) => Ok(Some(digest)),
                    Err(Status::NotFound(_)) => Ok(None),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            }
        }
        None => Ok(None),
    }
}
