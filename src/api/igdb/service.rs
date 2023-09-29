use crate::{
    api::FirestoreApi,
    documents::{
        Collection, CollectionType, Company, CompanyRole, GameCategory, GameDigest, GameEntry,
        Image, StoreEntry,
    },
    games::SteamDataApi,
    library::firestore,
    logging::IgdbCounters,
    util::rate_limiter::RateLimiter,
    Status,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing::{error, instrument, trace_span, Instrument};

use super::{
    backend::post,
    docs::{self, IgdbGameShort},
    ranking,
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

        let result: Vec<IgdbGameShort> = post(
            &connection,
            GAMES_ENDPOINT,
            &format!("fields id, name, first_release_date, aggregated_rating, category, version_parent, platforms, cover.image_id; where id={id};"),
        )
        .await?;

        match result.into_iter().next() {
            Some(igdb_game) => {
                match igdb_game.platforms.contains(&6)
                    || igdb_game.platforms.contains(&13)
                    || igdb_game.platforms.contains(&14)
                    || igdb_game.platforms.is_empty()
                {
                    true => Ok(GameDigest {
                        id: igdb_game.id,
                        name: igdb_game.name,
                        release_date: igdb_game.first_release_date,
                        rating: igdb_game.aggregated_rating,
                        category: match igdb_game.version_parent {
                            Some(_) => GameCategory::Version,
                            None => GameCategory::from(igdb_game.category),
                        },
                        cover: match igdb_game.cover {
                            Some(cover) => Some(cover.image_id),
                            None => None,
                        },
                        ..Default::default()
                    }),
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
        igdb_game: IgdbGame,
    ) -> Result<GameDigest, Status> {
        match resolve_game_digest(self.connection()?, firestore, igdb_game).await {
            Ok(game_entry) => Ok(GameDigest::from(game_entry)),
            Err(e) => Err(e),
        }
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
        IgdbCounters::resolve(&igdb_game);

        {
            let mut firestore = firestore.lock().unwrap();
            firestore.validate();
        }
        let connection = self.connection()?;

        let mut game_entry =
            resolve_game_digest(Arc::clone(&connection), Arc::clone(&firestore), igdb_game).await?;
        resolve_game_info(connection, &mut game_entry).await?;

        let steam = SteamDataApi::new();
        if let Err(e) = steam.retrieve_steam_data(&mut game_entry).await {
            error!("Failed to retrieve SteamData for '{}' {e}", game_entry.name);
        }

        if let Err(e) = firestore::games::write(&firestore.lock().unwrap(), &game_entry) {
            error!("Failed to save '{}' in Firestore: {e}", game_entry.name);
        }
        update_companies(Arc::clone(&firestore), &game_entry);
        update_collections(Arc::clone(&firestore), &game_entry);

        Ok(game_entry)
    }
}

/// Make sure that any companies involved in the game are updated to include it.
fn update_companies(firestore: Arc<Mutex<FirestoreApi>>, game_entry: &GameEntry) {
    let mut firestore = firestore.lock().unwrap();
    firestore.validate();

    let involved_companies: Vec<_> = vec![&game_entry.developers, &game_entry.publishers]
        .into_iter()
        .flatten()
        .map(|company| company)
        .collect();

    let mut companies: HashMap<_, _> = involved_companies
        .iter()
        .map(
            |involved_company| match firestore::companies::read(&firestore, involved_company.id) {
                Ok(company) => Some(company),
                Err(Status::NotFound(_)) => Some(Company {
                    id: involved_company.id,
                    name: involved_company.name.clone(),
                    slug: involved_company.slug.clone(),
                    ..Default::default()
                }),
                Err(e) => {
                    error!("{e}");
                    None
                }
            },
        )
        .filter_map(|e| e)
        .map(|e| (e.id, e))
        .collect();

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
                        company
                            .developed
                            .push(GameDigest::short_digest(&game_entry));
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
                        company
                            .published
                            .push(GameDigest::short_digest(&game_entry));
                        write_back.insert(company.id);
                    }
                }
                _ => {}
            }
        }
    }

    for id in write_back {
        if let Err(e) = firestore::companies::write(&firestore, &companies.get(&id).unwrap()) {
            error!("{e}")
        }
    }
}

/// Make sure that any collections / franchieses in the game are updated to
/// include it.
fn update_collections(firestore: Arc<Mutex<FirestoreApi>>, game_entry: &GameEntry) {
    let mut firestore = firestore.lock().unwrap();
    firestore.validate();

    for (collections, collection_type) in [
        (&game_entry.collections, CollectionType::Collection),
        (&game_entry.franchises, CollectionType::Franchise),
    ] {
        for collection in collections {
            match read_collection(&firestore, collection_type, collection.id) {
                Ok(mut collection) => {
                    match collection
                        .games
                        .iter()
                        .find(|game| game.id == game_entry.id)
                    {
                        Some(_) => continue,
                        None => {
                            // Game was missing from Collection.
                            collection.games.push(GameDigest::short_digest(&game_entry));
                            if let Err(e) =
                                write_collection(&firestore, collection_type, &collection)
                            {
                                error!("{e}")
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
                        games: vec![GameDigest::short_digest(&game_entry)],
                        ..Default::default()
                    };
                    if let Err(e) = write_collection(&firestore, collection_type, &collection) {
                        error!("{e}")
                    }
                }
                Err(e) => error!("{e}"),
            }
        }
    }
}

fn read_collection(
    firestore: &FirestoreApi,
    collection_type: CollectionType,
    id: u64,
) -> Result<Collection, Status> {
    match collection_type {
        CollectionType::Collection => firestore::collections::read(&firestore, id),
        CollectionType::Franchise => firestore::franchises::read(&firestore, id),
        CollectionType::Null => Err(Status::invalid_argument("invalid collection type")),
    }
}

fn write_collection(
    firestore: &FirestoreApi,
    collection_type: CollectionType,
    collection: &Collection,
) -> Result<(), Status> {
    match collection_type {
        CollectionType::Collection => firestore::collections::write(&firestore, &collection),
        CollectionType::Franchise => firestore::franchises::write(&firestore, &collection),
        CollectionType::Null => Err(Status::invalid_argument("invalid collection type")),
    }
}

pub const TWITCH_OAUTH_URL: &str = "https://id.twitch.tv/oauth2/token";

#[derive(Debug, Serialize, Deserialize)]
struct TwitchOAuthResponse {
    access_token: String,
    expires_in: i32,
}
