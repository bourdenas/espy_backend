use std::sync::Arc;

use crate::{
    api::{FirestoreApi, IgdbApi},
    documents::GameDigest,
    library::firestore,
    Status,
};
use tracing::{info, instrument};

pub struct Reconciler;

impl Reconciler {
    /// Returns a `GameDigest` from IGDB matching the `title`.
    #[instrument(level = "trace", skip(firestore, igdb))]
    pub async fn match_by_title(
        firestore: Arc<FirestoreApi>,
        igdb: &IgdbApi,
        title: &str,
    ) -> Result<Option<GameDigest>, Status> {
        info!("Searching by title '{}'", title);

        let candidates = igdb.search_by_title(title).await?;
        match candidates.into_iter().next() {
            Some(igdb_game) => match firestore::games::read(&firestore, igdb_game.id).await {
                Ok(game_entry) => Ok(Some(GameDigest::from(game_entry))),
                Err(Status::NotFound(_)) => {
                    match igdb.resolve_digest(&firestore, igdb_game).await {
                        Ok(digest) => Ok(Some(digest)),
                        Err(Status::NotFound(_)) => Ok(None),
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            },
            None => Ok(None),
        }
    }
}
