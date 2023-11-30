use crate::{
    api::FirestoreApi,
    documents::{
        Collection, CollectionType, Company, CompanyRole, GameDigest, GameEntry, Image, StoreEntry,
    },
    games::SteamDataApi,
    library::firestore,
    logging::{IgdbCounters, IgdbResolveCounter},
    util::rate_limiter::RateLimiter,
    Status,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
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
        match resolve_game_digest(&connection, firestore, igdb_game).await {
            Ok(game_entry) => Ok(GameDigest::from(game_entry)),
            Err(e) => Err(e),
        }
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
        counter.log(&game_entry);

        let steam = SteamDataApi::new();
        if let Err(e) = steam.retrieve_steam_data(&mut game_entry).await {
            warn!("Failed to retrieve SteamData for '{}' {e}", game_entry.name);
        }

        if let Err(e) = firestore::games::write(&firestore, &mut game_entry).await {
            warn!("Failed to save '{}' in Firestore: {e}", game_entry.name);
        }
        update_companies(Arc::clone(&firestore), &game_entry).await;
        update_collections(Arc::clone(&firestore), &game_entry).await;

        Ok(game_entry)
    }
}

/// Make sure that any companies involved in the game are updated to include it.
async fn update_companies(firestore: Arc<FirestoreApi>, game_entry: &GameEntry) {
    let involved_companies: Vec<_> = vec![&game_entry.developers, &game_entry.publishers]
        .into_iter()
        .flatten()
        .map(|company| company)
        .collect();

    let mut companies: HashMap<u64, Company> = HashMap::new();
    for involved_company in &involved_companies {
        let company = match firestore::companies::read(&firestore, involved_company.id).await {
            Ok(company) => Some(company),
            Err(Status::NotFound(_)) => Some(Company {
                id: involved_company.id,
                name: involved_company.name.clone(),
                slug: involved_company.slug.clone(),
                ..Default::default()
            }),
            Err(e) => {
                warn!("{e}");
                None
            }
        };
        if let Some(company) = company {
            companies.insert(company.id, company);
        }
    }

    let mut write_back = HashSet::new();
    for involved_company in involved_companies {
        if let Some(company) = companies.get_mut(&involved_company.id) {
            match involved_company.role {
                CompanyRole::Developer => {
                    if company
                        .developed
                        .iter()
                        .all(|game| game.id != game_entry.id)
                    {
                        // Game was missing from Company.
                        company.developed.push(GameDigest::from(game_entry.clone()));
                        write_back.insert(company.id);
                    }
                }
                CompanyRole::Publisher => {
                    if company
                        .published
                        .iter()
                        .all(|game| game.id != game_entry.id)
                    {
                        // Game was missing from Company.
                        company.published.push(GameDigest::from(game_entry.clone()));
                        write_back.insert(company.id);
                    }
                }
                _ => {}
            }
        }
    }

    for id in write_back {
        if let Err(e) = firestore::companies::write(&firestore, &companies.get(&id).unwrap()).await
        {
            warn!("{e}")
        }
    }
}

/// Make sure that any collections / franchieses in the game are updated to
/// include it.
async fn update_collections(firestore: Arc<FirestoreApi>, game_entry: &GameEntry) {
    for (collections, collection_type) in [
        (&game_entry.collections, CollectionType::Collection),
        (&game_entry.franchises, CollectionType::Franchise),
    ] {
        for collection in collections {
            match read_collection(&firestore, collection_type, collection.id).await {
                Ok(mut collection) => {
                    match collection
                        .games
                        .iter()
                        .find(|game| game.id == game_entry.id)
                    {
                        Some(_) => continue,
                        None => {
                            // Game was missing from Collection.
                            collection.games.push(GameDigest::from(game_entry.clone()));
                            if let Err(e) =
                                write_collection(&firestore, collection_type, &collection).await
                            {
                                warn!("{e}")
                            }
                        }
                    }
                }
                Err(Status::NotFound(_)) => {
                    // Collection was missing entirely.
                    let collection = Collection {
                        id: collection.id,
                        name: collection.name.clone(),
                        slug: collection.slug.clone(),
                        games: vec![GameDigest::from(game_entry.clone())],
                        ..Default::default()
                    };
                    if let Err(e) = write_collection(&firestore, collection_type, &collection).await
                    {
                        warn!("{e}")
                    }
                }
                Err(e) => warn!("{e}"),
            }
        }
    }
}

async fn read_collection(
    firestore: &FirestoreApi,
    collection_type: CollectionType,
    id: u64,
) -> Result<Collection, Status> {
    match collection_type {
        CollectionType::Collection => firestore::collections::read(&firestore, id).await,
        CollectionType::Franchise => firestore::franchises::read(&firestore, id).await,
        CollectionType::Null => Err(Status::invalid_argument("invalid collection type")),
    }
}

async fn write_collection(
    firestore: &FirestoreApi,
    collection_type: CollectionType,
    collection: &Collection,
) -> Result<(), Status> {
    match collection_type {
        CollectionType::Collection => firestore::collections::write(&firestore, &collection).await,
        CollectionType::Franchise => firestore::franchises::write(&firestore, &collection).await,
        CollectionType::Null => Err(Status::invalid_argument("invalid collection type")),
    }
}

pub const TWITCH_OAUTH_URL: &str = "https://id.twitch.tv/oauth2/token";

#[derive(Debug, Serialize, Deserialize)]
struct TwitchOAuthResponse {
    access_token: String,
    expires_in: i32,
}
