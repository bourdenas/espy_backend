use std::sync::Arc;

use crate::{
    api::FirestoreApi,
    documents::{GameDigest, GameEntry, IgdbGame},
    library::firestore,
    Status,
};
use itertools::Itertools;
use tracing::instrument;

use super::{connection::IgdbConnection, endpoints, ranking, request::post};

pub struct IgdbSearch {
    connection: Arc<IgdbConnection>,
}

impl IgdbSearch {
    pub fn new(connection: Arc<IgdbConnection>) -> IgdbSearch {
        IgdbSearch { connection }
    }

    /// Returns `GameDigest` for candidates matching the `title` in IGDB.
    ///
    /// Relies on finding matches games in Firestore. For those not found in
    /// Firestore it generates a basic GameDigest that lack info, e.g. cover.
    #[instrument(level = "trace", skip(self, firestore))]
    pub async fn search_by_title(
        &self,
        firestore: &FirestoreApi,
        title: &str,
    ) -> Result<Vec<GameDigest>, Status> {
        let candidates = self.match_by_title(title).await?;
        let candidate_ids = candidates.iter().map(|e| e.id).collect_vec();

        let result = firestore::games::batch_read(&firestore, &candidate_ids).await?;

        Ok(candidates
            .into_iter()
            .map(|igdb_game| {
                match result
                    .documents
                    .iter()
                    .find(|game_entry| game_entry.id == igdb_game.id)
                {
                    Some(game) => GameDigest::from(game.clone()),
                    _ => GameDigest::from(GameEntry::from(igdb_game)),
                }
            })
            .collect_vec())
    }

    /// Returns IgdbGames that match the `title` by searching in IGDB.
    #[instrument(level = "trace", skip(self))]
    async fn match_by_title(&self, title: &str) -> Result<Vec<IgdbGame>, Status> {
        Ok(ranking::sorted_by_relevance(
            title,
            self.search(title).await?,
        ))
    }

    #[instrument(level = "trace", skip(self))]
    async fn search(&self, title: &str) -> Result<Vec<IgdbGame>, Status> {
        let title = title.replace("\"", "");
        post::<Vec<IgdbGame>>(
            &self.connection,
            endpoints::GAMES,
            &format!("search \"{title}\"; fields *; where platforms = (6,13);"),
        )
        .await
    }
}
