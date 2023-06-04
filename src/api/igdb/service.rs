use crate::{
    api::FirestoreApi,
    documents::{GameDigest, GameEntry, Image, StoreEntry},
    games::SteamDataApi,
    library::firestore,
    util::rate_limiter::RateLimiter,
    Status,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing::{error, info, instrument, trace_span, Instrument};

use super::{
    backend::post,
    docs, ranking,
    resolve::{
        get_cover, resolve_game_digest, resolve_game_info, EXTERNAL_GAMES_ENDPOINT, GAMES_ENDPOINT,
    },
    IgdbConnection, IgdbGame,
};

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
            None => Err(Status::internal(
                "Connection with IGDB was not established.",
            )),
        }
    }

    /// Returns a GameEntry based on its IGDB `id`.
    ///
    /// The returned GameEntry is a shallow lookup. Reference ids are not
    /// followed up and thus it is not fully resolved.
    #[instrument(level = "trace", skip(self))]
    pub async fn get(&self, id: u64) -> Result<IgdbGame, Status> {
        let connection = self.connection()?;
        let result: Vec<IgdbGame> = post(
            &connection,
            GAMES_ENDPOINT,
            &format!("fields *; where id={id};"),
        )
        .await?;

        match result.into_iter().next() {
            Some(igdb_game) => Ok(igdb_game),
            None => Err(Status::not_found(format!(
                "IgdbGame with id={id} was not found."
            ))),
        }
    }

    /// Returns a GameDigest based on its IGDB `id`.
    ///
    /// This returns a short GameDigest that only resolves the game cover image.
    /// Only PC games are retrieved through this API.
    #[instrument(level = "trace", skip(self))]
    pub async fn get_short_digest(&self, id: u64) -> Result<GameDigest, Status> {
        let connection = self.connection()?;

        let result: Vec<IgdbGame> = post(
            &connection,
            GAMES_ENDPOINT,
            &format!("fields *; where id={id};"),
        )
        .await?;

        match result.into_iter().next() {
            Some(igdb_game) => {
                match igdb_game.platforms.contains(&6)
                    || igdb_game.platforms.contains(&13)
                    || igdb_game.platforms.contains(&14)
                    || igdb_game.platforms.is_empty()
                {
                    true => {
                        let cover = match igdb_game.cover {
                            Some(cover_id) => get_cover(&connection, cover_id).await?,
                            None => None,
                        };

                        let mut game_entry = GameEntry::from(igdb_game);
                        game_entry.cover = cover;
                        Ok(GameDigest::short_digest(game_entry))
                    }
                    false => Err(Status::not_found(format!(
                        "IgdbGame '{}' is not a PC game.",
                        igdb_game.name,
                    ))),
                }
            }
            None => Err(Status::not_found(format!(
                "IgdbGame with id={id} was not found."
            ))),
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn get_cover(&self, id: u64) -> Result<Option<Image>, Status> {
        let connection = self.connection()?;
        get_cover(&connection, id).await
    }

    /// Returns a GameDigest for an IgdbGame.
    ///
    /// This returns a full GameDigest that resolves all its fields.
    #[instrument(level = "trace", skip(self, firestore))]
    pub async fn get_digest(
        &self,
        firestore: Arc<Mutex<FirestoreApi>>,
        igdb_game: &IgdbGame,
    ) -> Result<GameEntry, Status> {
        resolve_game_digest(self.connection()?, firestore, igdb_game).await
    }

    /// Returns a GameEntry based on external id info in IGDB.
    ///
    /// The returned GameEntry is a shallow lookup. Reference ids are not
    /// followed up and thus it is not fully resolved.
    #[instrument(level = "trace", skip(self))]
    pub async fn get_by_store_entry(
        &self,
        store_entry: &StoreEntry,
    ) -> Result<Option<GameEntry>, Status> {
        let category: u8 = match store_entry.storefront_name.as_ref() {
            "steam" => 1,
            "gog" => 5,
            // "egs" => 26,
            "egs" => return Ok(None),
            _ => return Ok(None),
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
            Some(external_game) => Ok(Some(GameEntry::from(self.get(external_game.game).await?))),
            None => Ok(None),
        }
    }

    /// Returns candidate GameEntries by searching IGDB based on game title.
    ///
    /// The returned GameEntries are shallow lookups. Reference ids are not
    /// followed up and thus they are not fully resolved.
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
                                error!("Failed to retrieve cover: {e}");
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
    pub async fn expand_bundle(&self, bundle_id: u64) -> Result<Vec<IgdbGame>, Status> {
        let connection = self.connection()?;
        post::<Vec<IgdbGame>>(
            &connection,
            GAMES_ENDPOINT,
            &format!("fields *; where bundles = ({bundle_id});"),
        )
        .await
    }

    #[instrument(
        level = "trace",
        skip(self, firestore, igdb_game)
        fields(
            game_id = %igdb_game.id,
            title = %igdb_game.name
        )
    )]
    pub async fn resolve(
        &self,
        firestore: Arc<Mutex<FirestoreApi>>,
        igdb_game: IgdbGame,
    ) -> Result<GameEntry, Status> {
        info!(
            "Resolving in IGDB '{}' ({})",
            &igdb_game.name, &igdb_game.id
        );

        {
            let mut firestore = firestore.lock().unwrap();
            firestore.validate();
        }
        let connection = self.connection()?;

        let mut game_entry =
            resolve_game_digest(Arc::clone(&connection), Arc::clone(&firestore), &igdb_game)
                .await?;
        resolve_game_info(connection, igdb_game, &mut game_entry).await?;

        let steam = SteamDataApi::new();
        if let Err(e) = steam.retrieve_steam_data(&mut game_entry).await {
            error!("Failed to retrieve SteamData for '{}' {e}", game_entry.name);
        }

        if let Err(e) = firestore::games::write(&firestore.lock().unwrap(), &game_entry) {
            error!("Failed to save '{}' in Firestore: {e}", game_entry.name);
        }

        Ok(game_entry)
    }
}

pub const TWITCH_OAUTH_URL: &str = "https://id.twitch.tv/oauth2/token";

#[derive(Debug, Serialize, Deserialize)]
struct TwitchOAuthResponse {
    access_token: String,
    expires_in: i32,
}
