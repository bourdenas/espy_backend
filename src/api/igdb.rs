use super::igdb_docs::{self, ExternalGame, IgdbGame, InvolvedCompany};
use crate::{
    documents::{Annotation, GameEntry, Image, StoreEntry, Website, WebsiteAuthority},
    util::rate_limiter::RateLimiter,
    Status,
};
use async_recursion::async_recursion;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, instrument, trace_span, Instrument};

pub struct IgdbApi {
    secret: String,
    client_id: String,
    state: Option<Arc<IgdbApiState>>,
}

#[derive(Debug)]
struct IgdbApiState {
    client_id: String,
    oauth_token: String,
    qps: RateLimiter,
}

impl IgdbApi {
    pub fn new(client_id: &str, secret: &str) -> IgdbApi {
        IgdbApi {
            secret: String::from(secret),
            client_id: String::from(client_id),
            state: None,
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

        self.state = Some(Arc::new(IgdbApiState {
            client_id: self.client_id.clone(),
            oauth_token: resp.access_token,
            qps: RateLimiter::new(4),
        }));

        Ok(())
    }

    /// Returns matching candidates by searching based on game title from the
    /// igdb/games endpoint.
    ///
    /// Returns barebone candidates with not many of the relevant IGDB fields
    /// populated to save on extra queries.
    #[instrument(level = "trace", skip(self))]
    pub async fn search_by_title(&self, title: &str) -> Result<Vec<IgdbGame>, Status> {
        let igdb_state = match &self.state {
            Some(state) => Arc::clone(state),
            None => {
                return Err(Status::invalid_argument(
                    "Connection with IGDB was not established.",
                ));
            }
        };

        Ok(post(
            igdb_state,
            GAMES_ENDPOINT,
            &format!("search \"{title}\"; fields *;"),
        )
        .await?)
    }

    /// Returns a fully resolved IGDB Game based on the provided storefront
    /// entry if found in IGDB.
    #[instrument(level = "trace", skip(self))]
    pub async fn match_store_entry(
        &self,
        store_entry: &StoreEntry,
    ) -> Result<Option<GameEntry>, Status> {
        let igdb_state = match &self.state {
            Some(state) => Arc::clone(state),
            None => {
                return Err(Status::invalid_argument(
                    "Connection with IGDB was not established.",
                ));
            }
        };

        let category: u8 = match store_entry.storefront_name.as_ref() {
            "steam" => 1,
            "gog" => 5,
            // "egs" => 26,
            "egs" => return Ok(None),
            _ => return Ok(None),
        };

        let result: Vec<ExternalGame> = post(
            Arc::clone(&igdb_state),
            EXTERNAL_GAMES_ENDPOINT,
            &format!(
                "fields *; where uid = \"{}\" & category = {category};",
                store_entry.id
            ),
        )
        .await?;

        match result.into_iter().next() {
            Some(external_game) => retrieve_game(igdb_state, external_game.game).await,
            None => Ok(None),
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn get_game_by_id(&self, id: u64) -> Result<Option<GameEntry>, Status> {
        let igdb_state = match &self.state {
            Some(state) => Arc::clone(state),
            None => {
                return Err(Status::invalid_argument(
                    "Connection with IGDB was not established.",
                ));
            }
        };

        retrieve_game(igdb_state, id).await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn get_cover(&self, id: u64) -> Result<Option<Image>, Status> {
        let igdb_state = match &self.state {
            Some(state) => Arc::clone(state),
            None => {
                return Err(Status::invalid_argument(
                    "Connection with IGDB was not established.",
                ));
            }
        };

        get_cover(igdb_state, id).await
    }
}

/// Returns a fully resolved IGDB Game matching the input IGDB Game id.
#[instrument(level = "trace", skip(igdb_state))]
async fn retrieve_game(
    igdb_state: Arc<IgdbApiState>,
    id: u64,
) -> Result<Option<GameEntry>, Status> {
    let result: Vec<IgdbGame> = post(
        Arc::clone(&igdb_state),
        GAMES_ENDPOINT,
        &format!("fields *; where id={id};"),
    )
    .await?;

    match result.into_iter().next() {
        Some(game) => match retrieve_game_info(igdb_state, game).await {
            Ok(game) => Ok(Some(game)),
            Err(e) => Err(e),
        },
        None => Ok(None),
    }
}

/// Retrieves igdb.Game fields that are relevant to espy. For instance, cover
/// images, screenshots, expansions, etc.
///
/// IGDB returns Game entries only with relevant IDs for such items that need
/// subsequent lookups in corresponding IGDB tables.
#[async_recursion]
#[instrument(
    level = "trace",
    skip(igdb_state, igdb_game),
    fields(
        game_id = %igdb_game.id,
        game_name = %igdb_game.name,
    )
)]
async fn retrieve_game_info(
    igdb_state: Arc<IgdbApiState>,
    igdb_game: IgdbGame,
) -> Result<GameEntry, Status> {
    let game = GameEntry {
        id: igdb_game.id,
        name: igdb_game.name,
        summary: igdb_game.summary,
        storyline: igdb_game.storyline,
        release_date: igdb_game.first_release_date,

        versions: igdb_game.bundles,
        parent: match igdb_game.parent_game {
            Some(parent) => Some(parent),
            None => match igdb_game.version_parent {
                Some(parent) => Some(parent),
                None => None,
            },
        },

        websites: vec![Website {
            url: igdb_game.url,
            authority: WebsiteAuthority::Igdb,
        }],

        ..Default::default()
    };

    let game = Arc::new(Mutex::new(game));

    let mut handles: Vec<JoinHandle<Result<(), Status>>> = vec![];
    if let Some(cover) = igdb_game.cover {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                game.lock().unwrap().cover = get_cover(igdb_state, cover).await?;
                Ok(())
            }
            .instrument(trace_span!("spawn_get_cover")),
        ));
    }
    if let Some(collection) = igdb_game.collection {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                if let Some(collection) = get_collection(igdb_state, collection).await? {
                    game.lock().unwrap().collections.push(collection);
                }
                Ok(())
            }
            .instrument(trace_span!("spawn_get_collection")),
        ));
    }
    if igdb_game.franchises.len() > 0 {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                let franchise = get_franchises(igdb_state, &igdb_game.franchises).await?;
                game.lock().unwrap().collections.extend(franchise);
                Ok(())
            }
            .instrument(trace_span!("spawn_get_franchises")),
        ));
    }
    if igdb_game.involved_companies.len() > 0 {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                game.lock().unwrap().companies =
                    get_companies(igdb_state, &igdb_game.involved_companies).await?;
                Ok(())
            }
            .instrument(trace_span!("spawn_get_companies")),
        ));
    }
    if igdb_game.artworks.len() > 0 {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                game.lock().unwrap().artwork = get_artwork(igdb_state, &igdb_game.artworks).await?;
                Ok(())
            }
            .instrument(trace_span!("spawn_get_artwork")),
        ));
    }
    if igdb_game.screenshots.len() > 0 {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                game.lock().unwrap().screenshots =
                    get_screenshots(igdb_state, &igdb_game.screenshots).await?;
                Ok(())
            }
            .instrument(trace_span!("spawn_get_screenshots")),
        ));
    }
    if igdb_game.websites.len() > 0 {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                let websites = get_websites(igdb_state, &igdb_game.websites).await?;
                game.lock()
                    .unwrap()
                    .websites
                    .extend(websites.into_iter().map(|website| Website {
                        url: website.url,
                        authority: match website.category {
                            1 => WebsiteAuthority::Official,
                            3 => WebsiteAuthority::Wikipedia,
                            9 => WebsiteAuthority::Youtube,
                            13 => WebsiteAuthority::Steam,
                            16 => WebsiteAuthority::Egs,
                            17 => WebsiteAuthority::Gog,
                            _ => WebsiteAuthority::Null,
                        },
                    }));
                Ok(())
            }
            .instrument(trace_span!("spawn_get_websites")),
        ));
    }

    for expansion in igdb_game.expansions.into_iter() {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                if let Some(expansion) = retrieve_game(igdb_state, expansion).await? {
                    game.lock().unwrap().expansions.push(expansion);
                }
                Ok(())
            }
            .instrument(trace_span!("spawn_get_expansions")),
        ));
    }
    for dlc in igdb_game.dlcs.into_iter() {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                if let Some(dlc) = retrieve_game(igdb_state, dlc).await? {
                    game.lock().unwrap().dlcs.push(dlc);
                }
                Ok(())
            }
            .instrument(trace_span!("spawn_get_dlcs")),
        ));
    }
    for remake in igdb_game.remakes.into_iter() {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                if let Some(remake) = retrieve_game(igdb_state, remake).await? {
                    game.lock().unwrap().remakes.push(remake);
                }
                Ok(())
            }
            .instrument(trace_span!("spawn_get_remakes")),
        ));
    }
    for remaster in igdb_game.remasters.into_iter() {
        let igdb_state = Arc::clone(&igdb_state);
        let game = Arc::clone(&game);
        handles.push(tokio::spawn(
            async move {
                if let Some(remaster) = retrieve_game(igdb_state, remaster).await? {
                    game.lock().unwrap().remasters.push(remaster);
                }
                Ok(())
            }
            .instrument(trace_span!("spawn_get_remasters")),
        ));
    }

    for result in futures::future::join_all(handles).await {
        match result {
            Ok(result) => {
                if let Err(e) = result {
                    return Err(e);
                }
            }
            Err(e) => return Err(Status::Internal(format!("{}", e))),
        }
    }

    Ok(Arc::try_unwrap(game).unwrap().into_inner().unwrap())
}

/// Returns game image cover based on id from the igdb/covers endpoint.
#[instrument(level = "trace", skip(igdb_state))]
async fn get_cover(igdb_state: Arc<IgdbApiState>, id: u64) -> Result<Option<Image>, Status> {
    let result: Vec<Image> = post(
        igdb_state,
        COVERS_ENDPOINT,
        &format!("fields *; where id={id};"),
    )
    .await?;

    Ok(result.into_iter().next())
}

/// Returns game screenshots based on id from the igdb/screenshots endpoint.
#[instrument(level = "trace", skip(igdb_state))]
async fn get_artwork(igdb_state: Arc<IgdbApiState>, ids: &[u64]) -> Result<Vec<Image>, Status> {
    Ok(post(
        igdb_state,
        ARTWORKS_ENDPOINT,
        &format!(
            "fields *; where id = ({});",
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",")
        ),
    )
    .await?)
}

/// Returns game screenshots based on id from the igdb/screenshots endpoint.
#[instrument(level = "trace", skip(igdb_state))]
async fn get_screenshots(igdb_state: Arc<IgdbApiState>, ids: &[u64]) -> Result<Vec<Image>, Status> {
    Ok(post(
        igdb_state,
        SCREENSHOTS_ENDPOINT,
        &format!(
            "fields *; where id = ({});",
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",")
        ),
    )
    .await?)
}

/// Returns game websites based on id from the igdb/websites endpoint.
#[instrument(level = "trace", skip(igdb_state))]
async fn get_websites(
    igdb_state: Arc<IgdbApiState>,
    ids: &[u64],
) -> Result<Vec<igdb_docs::Website>, Status> {
    Ok(post(
        igdb_state,
        WEBSITES_ENDPOINT,
        &format!(
            "fields *; where id = ({});",
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",")
        ),
    )
    .await?)
}

/// Returns game collection based on id from the igdb/collections endpoint.
#[instrument(level = "trace", skip(igdb_state))]
async fn get_collection(
    igdb_state: Arc<IgdbApiState>,
    id: u64,
) -> Result<Option<Annotation>, Status> {
    let result: Vec<Annotation> = post(
        igdb_state,
        COLLECTIONS_ENDPOINT,
        &format!("fields *; where id={id};"),
    )
    .await?;

    Ok(result.into_iter().next())
}

/// Returns game franchices based on id from the igdb/frachises endpoint.
#[instrument(level = "trace", skip(igdb_state))]
async fn get_franchises(
    igdb_state: Arc<IgdbApiState>,
    ids: &[u64],
) -> Result<Vec<Annotation>, Status> {
    Ok(post(
        igdb_state,
        FRANCHISES_ENDPOINT,
        &format!(
            "fields *; where id = ({});",
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",")
        ),
    )
    .await?)
}

/// Returns game companies involved in the making of the game.
#[instrument(level = "trace", skip(igdb_state))]
async fn get_companies(
    igdb_state: Arc<IgdbApiState>,
    ids: &[u64],
) -> Result<Vec<Annotation>, Status> {
    // Collect all involved companies for a game entry.
    let involved_companies: Vec<InvolvedCompany> = post(
        Arc::clone(&igdb_state),
        INVOLVED_COMPANIES_ENDPOINT,
        &format!(
            "fields *; where id = ({});",
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
    )
    .await?;

    // Collect company data for involved companies.
    let companies: Vec<Annotation> = post::<Vec<Annotation>>(
        igdb_state,
        COMPANIES_ENDPOINT,
        &format!(
            "fields *; where id = ({});",
            involved_companies
                .iter()
                .map(|ic| match &ic.company {
                    Some(c) => c.to_string(),
                    None => "".to_string(),
                })
                // TODO: Due to incomplete IGDB data filtering can leave
                // no company ids involved which results to a bad
                // request to IGDB. Temporarily removing the developer
                // requirement until fixing properly.
                //
                // .filter_map(|ic| match ic.developer {
                //     true => match &ic.company {
                //         Some(c) => Some(c.id.to_string()),
                //         None => None,
                //     },
                //     false => None,
                // })
                .collect::<Vec<_>>()
                .join(",")
        ),
    )
    .await?
    .into_iter()
    .filter(|company| !company.name.is_empty())
    .collect();

    Ok(companies)
}

/// Sends a POST request to an IGDB service endpoint.
async fn post<T: DeserializeOwned>(
    igdb_state: Arc<IgdbApiState>,
    endpoint: &str,
    body: &str,
) -> Result<T, Status> {
    igdb_state.qps.wait();

    let uri = format!("{IGDB_SERVICE_URL}/{endpoint}/");
    let resp = reqwest::Client::new()
        .post(&uri)
        .header("Client-ID", &igdb_state.client_id)
        .header(
            "Authorization",
            format!("Bearer {}", &igdb_state.oauth_token),
        )
        .body(String::from(body))
        .send()
        .await?;

    let text = resp.text().await?;
    let resp = serde_json::from_str::<T>(&text).map_err(|_| {
        let msg = format!("Received unexpected response: {}", &text);
        error!(msg);
        Status::internal(msg)
    });

    resp
}

const TWITCH_OAUTH_URL: &str = "https://id.twitch.tv/oauth2/token";
const IGDB_SERVICE_URL: &str = "https://api.igdb.com/v4";
const GAMES_ENDPOINT: &str = "games";
const EXTERNAL_GAMES_ENDPOINT: &str = "external_games";
const COVERS_ENDPOINT: &str = "covers";
const FRANCHISES_ENDPOINT: &str = "franchises";
const COLLECTIONS_ENDPOINT: &str = "collections";
const ARTWORKS_ENDPOINT: &str = "artworks";
const SCREENSHOTS_ENDPOINT: &str = "screenshots";
const WEBSITES_ENDPOINT: &str = "websites";
const COMPANIES_ENDPOINT: &str = "companies";
const INVOLVED_COMPANIES_ENDPOINT: &str = "involved_companies";

#[derive(Debug, Serialize, Deserialize)]
struct TwitchOAuthResponse {
    access_token: String,
    expires_in: i32,
}
