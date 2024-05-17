use std::sync::Arc;

use crate::{
    api::{FirestoreApi, IgdbApi},
    documents::{GameDigest, GameEntry},
    library::firestore,
    Status,
};
use tracing::{info, instrument, trace_span, warn, Instrument};

use super::{
    backend::post,
    ranking,
    resolve::{get_cover, GAMES_ENDPOINT},
    IgdbGame,
};

pub struct IgdbSearch {
    igdb: Arc<IgdbApi>,
}

impl IgdbSearch {
    pub fn new(igdb: Arc<IgdbApi>) -> IgdbSearch {
        IgdbSearch { igdb }
    }

    /// Returns a `GameDigest` from IGDB matching the `title`.
    #[instrument(level = "trace", skip(self, firestore, igdb))]
    pub async fn match_by_title(
        &self,
        firestore: Arc<FirestoreApi>,
        igdb: &IgdbApi,
        title: &str,
    ) -> Result<Option<GameDigest>, Status> {
        info!("Searching by title '{}'", title);

        let candidates = self.search_by_title(title).await?;
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

    /// Returns IgdbGames that match the `title` by searching in IGDB.
    #[instrument(level = "trace", skip(self))]
    pub async fn search_by_title(&self, title: &str) -> Result<Vec<IgdbGame>, Status> {
        Ok(ranking::sorted_by_relevance(
            title,
            self.search(title).await?,
        ))
    }

    /// Returns candidate GameEntries by searching IGDB based on game title.
    ///
    /// The returned GameEntries are shallow lookups similar to
    /// `search_by_title()`, but have their cover image resolved.
    #[instrument(level = "trace", skip(self))]
    pub async fn search_by_title_with_cover(
        &self,
        title: &str,
        base_games_only: bool,
    ) -> Result<Vec<GameEntry>, Status> {
        let mut igdb_games = self.search(title).await?;
        if base_games_only {
            igdb_games.retain(|game| game.parent_game.is_none());
        }

        let igdb_games = ranking::sorted_by_relevance_with_threshold(title, igdb_games, 1.0);

        // TODO: get covers from firestore intead of IGDB.
        let connection = self.igdb.connection()?;
        let mut handles = vec![];
        for game in igdb_games {
            let connection = Arc::clone(&connection);
            handles.push(tokio::spawn(
                async move {
                    let cover = match game.cover {
                        Some(id) => match get_cover(&connection, id).await {
                            Ok(cover) => cover,
                            Err(e) => {
                                warn!("Failed to retrieve cover: {e}");
                                None
                            }
                        },
                        None => None,
                    };

                    let mut game_entry = GameEntry::from(game);
                    game_entry.cover = cover;
                    game_entry
                }
                .instrument(trace_span!("spawn_get_cover")),
            ));
        }

        Ok(futures::future::join_all(handles)
            .await
            .into_iter()
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>())
    }

    #[instrument(level = "trace", skip(self))]
    async fn search(&self, title: &str) -> Result<Vec<IgdbGame>, Status> {
        let title = title.replace("\"", "");
        let connection = self.igdb.connection()?;
        post::<Vec<IgdbGame>>(
            &connection,
            GAMES_ENDPOINT,
            &format!("search \"{title}\"; fields *; where platforms = (6,13);"),
        )
        .await
    }
}
