use crate::{
    api::FirestoreApi,
    documents::{GameDigest, GameEntry, Image, StoreEntry},
    library::firestore,
    logging::{IgdbCounters, IgdbResolveCounter},
    util::rate_limiter::RateLimiter,
    Status,
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tracing::{instrument, trace_span, warn, Instrument};

use super::{backend::post, docs, ranking, resolve::*, IgdbConnection, IgdbGame};

#[derive(Clone)]
pub struct IgdbApi {
    secret: String,
    client_id: String,
    connection: Option<Arc<IgdbConnection>>,
}

impl IgdbApi {
    pub fn new(client_id: &str, secret: &str) -> IgdbApi {
        IgdbApi {
            secret: String::from(secret),
            client_id: String::from(client_id),
            connection: None,
        }
    }

    /// Authenticate with twtich/igdb OAuth2 server and retrieve session token.
    /// Authentication is valid for the lifetime of this instane or until the
    /// retrieved token expires.
    pub async fn connect(&mut self) -> Result<(), Status> {
        let uri = format!(
            "{TWITCH_OAUTH_URL}?client_id={}&client_secret={}&grant_type=client_credentials",
            self.client_id, self.secret
        );

        let resp = reqwest::Client::new()
            .post(&uri)
            .send()
            .await?
            .json::<TwitchOAuthResponse>()
            .await?;

        self.connection = Some(Arc::new(IgdbConnection {
            client_id: self.client_id.clone(),
            oauth_token: resp.access_token,
            qps: RateLimiter::new(4, Duration::from_secs(1), 6),
        }));

        Ok(())
    }

    pub fn connection(&self) -> Result<Arc<IgdbConnection>, Status> {
        match &self.connection {
            Some(connection) => Ok(Arc::clone(connection)),
            None => {
                let status = Status::internal(
                    "Tried to access IGDB API without establishing a connection first.",
                );
                IgdbCounters::connection_fail(&status);
                Err(status)
            }
        }
    }

    /// Returns an IgdbGame based on its `id`.
    #[instrument(level = "trace", skip(self))]
    pub async fn get(&self, id: u64) -> Result<IgdbGame, Status> {
        let connection = self.connection()?;
        get_game(&connection, id).await
    }

    /// Returns an IgdbGame based on external id info in IGDB.
    #[instrument(level = "trace", skip(self))]
    pub async fn get_by_store_entry(&self, store_entry: &StoreEntry) -> Result<IgdbGame, Status> {
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

        let connection = self.connection()?;
        let result: Vec<docs::IgdbExternalGame> = post(
            &connection,
            EXTERNAL_GAMES_ENDPOINT,
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

        let connection = self.connection()?;
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
        let connection = self.connection()?;
        post::<Vec<IgdbGame>>(
            &connection,
            GAMES_ENDPOINT,
            &format!("search \"{title}\"; fields *; where platforms = (6,13,14);"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn get_cover(&self, id: u64) -> Result<Option<Image>, Status> {
        let connection = self.connection()?;
        get_cover(&connection, id).await
    }

    /// Returns IgdbGames included in the bundle of `bundle_id`.
    #[instrument(level = "trace", skip(self))]
    pub async fn expand_bundle(&self, bundle_id: u64) -> Result<Vec<IgdbGame>, Status> {
        let connection = self.connection()?;
        post::<Vec<IgdbGame>>(
            &connection,
            GAMES_ENDPOINT,
            &format!("fields *; where bundles = ({bundle_id});"),
        )
        .await
    }

    /// Returns a GameDigest for an IgdbGame.
    #[instrument(
        level = "trace",
        skip(self, firestore),
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
        let connection = self.connection()?;
        Ok(GameDigest::from(
            resolve_game_digest(&connection, firestore, igdb_game).await?,
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
        let connection = self.connection()?;

        let counter = IgdbResolveCounter::new();
        let mut game_entry = match resolve_game_digest(&connection, &firestore, igdb_game).await {
            Ok(entry) => entry,
            Err(status) => {
                counter.log_error(&status);
                return Err(status);
            }
        };
        match resolve_game_info(&connection, &firestore, &mut game_entry).await {
            Ok(()) => {}
            Err(status) => {
                counter.log_error(&status);
                return Err(status);
            }
        }

        if let Err(e) = firestore::games::write(&firestore, &mut game_entry).await {
            warn!("Failed to save '{}' in Firestore: {e}", game_entry.name);
        }

        counter.log(&game_entry);

        Ok(game_entry)
    }
}

pub const TWITCH_OAUTH_URL: &str = "https://id.twitch.tv/oauth2/token";

#[derive(Debug, Serialize, Deserialize)]
struct TwitchOAuthResponse {
    access_token: String,
    expires_in: i32,
}
