use std::sync::Arc;
use tracing::instrument;

use crate::{
    api::FirestoreApi,
    documents::{GameDigest, GameEntry, IgdbExternalGame, IgdbGame, StoreEntry},
    logging::IgdbResolveCounter,
    Status,
};

use super::{backend::post, endpoints, resolve::*, IgdbConnection};

#[derive(Clone)]
pub struct IgdbApi {
    connection: Arc<IgdbConnection>,
}

impl IgdbApi {
    pub fn new(connection: Arc<IgdbConnection>) -> IgdbApi {
        IgdbApi { connection }
    }

    /// Returns an IgdbGame based on its `id`.
    #[instrument(level = "trace", skip(self))]
    pub async fn get(&self, id: u64) -> Result<IgdbGame, Status> {
        get_game(&self.connection, id).await
    }

    /// Returns a GameDigest for an IgdbGame.
    #[instrument(
        level = "trace",
        skip(self, firestore, igdb_game),
        fields(
            game_id = %igdb_game.id,
            title = %igdb_game.name
        )
    )]
    pub async fn resolve_digest(
        &self,
        firestore: &FirestoreApi,
        igdb_game: IgdbGame,
    ) -> Result<GameDigest, Status> {
        Ok(GameDigest::from(
            resolve_game_digest(&self.connection, firestore, igdb_game).await?,
        ))
    }

    #[instrument(
        level = "trace",
        skip(self, firestore, igdb_game),
        fields(
            game_id = %igdb_game.id,
            title = %igdb_game.name
        )
    )]
    pub async fn resolve(
        &self,
        firestore: Arc<FirestoreApi>,
        igdb_game: IgdbGame,
    ) -> Result<GameEntry, Status> {
        let counter = IgdbResolveCounter::new();
        let mut game_entry =
            match resolve_game_digest(&self.connection, &firestore, igdb_game).await {
                Ok(entry) => entry,
                Err(status) => {
                    counter.log_error(&status);
                    return Err(status);
                }
            };

        match resolve_game_info(&self.connection, &firestore, &mut game_entry).await {
            Ok(()) => {}
            Err(status) => {
                counter.log_error(&status);
                return Err(status);
            }
        }
        counter.log(&game_entry);

        Ok(game_entry)
    }

    /// Returns an IgdbGame based on external id info in IGDB.
    #[instrument(level = "trace", skip(self))]
    async fn get_by_store_entry(&self, store_entry: &StoreEntry) -> Result<IgdbGame, Status> {
        let category: u8 = match store_entry.storefront_name.as_ref() {
            "steam" => 1,
            "gog" => 5,
            // "egs" => 26,
            "egs" => return Err(Status::invalid_argument("'egs' store is not supported")),
            store => {
                return Err(Status::invalid_argument(format!(
                    "'{store}' store is not supported"
                )))
            }
        };

        let result: Vec<IgdbExternalGame> = post(
            &self.connection,
            endpoints::EXTERNAL_GAMES,
            &format!(
                "fields *; where uid = \"{}\" & category = {category};",
                store_entry.id
            ),
        )
        .await?;

        match result.into_iter().next() {
            Some(external_game) => Ok(self.get(external_game.game).await?),
            None => Err(Status::not_found(format!(
                "was not able to find a match for {:?}",
                store_entry
            ))),
        }
    }
}
