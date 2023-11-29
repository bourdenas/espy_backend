use crate::{
    api::FirestoreApi,
    documents::{
        CollectionDigest, CollectionType, CompanyDigest, CompanyRole, GameDigest, GameEntry, Image,
        Website, WebsiteAuthority,
    },
    library::firestore,
    Status,
};
use async_recursion::async_recursion;
use std::sync::Arc;
use tracing::instrument;

use super::{
    backend::post,
    docs::{self, IgdbInvolvedCompany},
    IgdbConnection, IgdbGame,
};

/// Returns an IgdbGame doc from IGDB for given game `id`.
///
/// Does not perform any lookups on tables beyond Game.
#[instrument(level = "trace", skip(connection))]
pub async fn get_game(connection: &IgdbConnection, id: u64) -> Result<IgdbGame, Status> {
    let result: Vec<IgdbGame> = post(
        connection,
        GAMES_ENDPOINT,
        &format!("fields *; where id={id};"),
    )
    .await?;

    match result.into_iter().next() {
        Some(igdb_game) => Ok(igdb_game),
        None => Err(Status::not_found(format!(
            "Failed to retrieve game with id={id}"
        ))),
    }
}

/// Returns a GameEntry from IGDB that can build a short GameDigest doc.
///
/// A short GameDigest resolves only the game cover.
#[instrument(level = "trace", skip(connection))]
pub async fn get_short_digest(connection: &IgdbConnection, id: u64) -> Result<GameDigest, Status> {
    let result: Vec<IgdbGame> = post(
        &connection,
        GAMES_ENDPOINT,
        &format!("fields *; where id={id};"),
    )
    .await?;

    match result.into_iter().next() {
        Some(igdb_game) => {
            let cover = match igdb_game.cover {
                Some(cover_id) => get_cover(&connection, cover_id).await?,
                None => None,
            };

            let mut game_entry = GameEntry::from(igdb_game);
            game_entry.cover = cover;
            Ok(GameDigest::from(game_entry))
        }
        None => Err(Status::not_found(format!(
            "IgdbGame with id={id} was not found."
        ))),
    }
}

/// Returns a GameEntry from IGDB that can build the GameDigest doc.
#[instrument(
    level = "trace",
    skip(connection, firestore, igdb_game)
    fields(
        game_id = %igdb_game.id,
        game_name = %igdb_game.name,
    )
)]
pub async fn resolve_game_digest(
    connection: Arc<IgdbConnection>,
    firestore: Arc<FirestoreApi>,
    igdb_game: IgdbGame,
) -> Result<GameEntry, Status> {
    let mut game_entry = GameEntry::from(igdb_game);
    let igdb_game = &game_entry.igdb_game;

    if let Some(cover) = igdb_game.cover {
        game_entry.cover = get_cover(&connection, cover).await?;
    }

    if let Some(collection) = igdb_game.collection {
        if let Some(collection) = get_collection(&connection, &firestore, collection).await? {
            game_entry.collections = vec![collection];
        }
    }

    let mut franchises = [
        match igdb_game.franchise {
            Some(id) => vec![id],
            None => vec![],
        },
        igdb_game.franchises.clone(),
    ]
    .concat();
    if !franchises.is_empty() {
        franchises.sort();
        franchises.dedup();
        game_entry
            .franchises
            .extend(get_franchises(&connection, &firestore, &franchises).await?);
    }

    if !igdb_game.involved_companies.is_empty() {
        let companies =
            get_involved_companies(&connection, &firestore, &igdb_game.involved_companies).await?;
        game_entry.developers = companies
            .iter()
            .filter(|company| match company.role {
                CompanyRole::Developer => true,
                _ => false,
            })
            // NOTE: drain_filter() would prevent the cloning.
            .map(|company| company.clone())
            .collect();
        game_entry.publishers = companies
            .into_iter()
            .filter(|company| match company.role {
                CompanyRole::Publisher => true,
                _ => false,
            })
            .collect();
    }

    if !igdb_game.genres.is_empty() {
        let igdb_genres = get_genres(&connection, &firestore, &igdb_game.genres).await?;
        game_entry.add_genres(&igdb_genres);
    }

    Ok(game_entry)
}

/// Returns a fully resolved GameEntry from IGDB that goes beyond the GameDigest doc.
#[async_recursion]
#[instrument(
    level = "trace",
    skip(connection, firestore, game_entry),
    fields(
        game_id = %game_entry.id,
        game_name = %game_entry.name,
    )
)]
pub async fn resolve_game_info(
    connection: Arc<IgdbConnection>,
    firestore: Arc<FirestoreApi>,
    game_entry: &mut GameEntry,
) -> Result<(), Status> {
    let igdb_game = &game_entry.igdb_game;

    if !igdb_game.keywords.is_empty() {
        game_entry.keywords = get_keywords(&connection, &firestore, &igdb_game.keywords).await?;
    }

    if !igdb_game.screenshots.is_empty() {
        if let Ok(screenshots) = get_screenshots(&connection, &igdb_game.screenshots).await {
            game_entry.screenshots = screenshots;
        }
    }
    if !igdb_game.artworks.is_empty() {
        if let Ok(artwork) = get_artwork(&connection, &igdb_game.artworks).await {
            game_entry.artwork = artwork;
        }
    }
    if igdb_game.websites.len() > 0 {
        if let Ok(websites) = get_websites(&connection, &igdb_game.websites).await {
            game_entry.websites.extend(
                websites
                    .into_iter()
                    .map(|website| Website {
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
                    })
                    .filter(|website| match website.authority {
                        WebsiteAuthority::Null => false,
                        _ => true,
                    }),
            );
        }
    }

    let parent_id = match igdb_game.parent_game {
        Some(parent) => Some(parent),
        None => match igdb_game.version_parent {
            Some(parent) => Some(parent),
            None => None,
        },
    };

    if let Some(id) = parent_id {
        if let Ok(game) = get_short_digest(&connection, id).await {
            game_entry.parent = Some(game);
        };
    }
    for id in igdb_game.expansions.iter() {
        if let Ok(game) = get_short_digest(&connection, *id).await {
            game_entry.expansions.push(game);
        };
    }
    for id in igdb_game.standalone_expansions.iter() {
        if let Ok(game) = get_short_digest(&connection, *id).await {
            game_entry.expansions.push(game);
        };
    }
    for id in igdb_game.dlcs.iter() {
        if let Ok(game) = get_short_digest(&connection, *id).await {
            game_entry.dlcs.push(game);
        };
    }
    for id in igdb_game.remakes.iter() {
        if let Ok(game) = get_short_digest(&connection, *id).await {
            game_entry.remakes.push(game);
        };
    }
    for id in igdb_game.remasters.iter() {
        if let Ok(game) = get_short_digest(&connection, *id).await {
            game_entry.remasters.push(game);
        };
    }

    Ok(())
}

/// Returns game image cover based on id from the igdb/covers endpoint.
#[instrument(level = "trace", skip(connection))]
pub async fn get_cover(connection: &IgdbConnection, id: u64) -> Result<Option<Image>, Status> {
    let result: Vec<Image> = post(
        connection,
        COVERS_ENDPOINT,
        &format!("fields *; where id={id};"),
    )
    .await?;

    Ok(result.into_iter().next())
}

/// Returns game genres based on id from the igdb/genres endpoint.
#[instrument(level = "trace", skip(connection, firestore))]
async fn get_genres(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    ids: &[u64],
) -> Result<Vec<String>, Status> {
    let mut genres = vec![];
    let mut missing = vec![];
    for id in ids {
        match firestore::genres::read(firestore, *id).await {
            Ok(genre) => genres.push(genre.name),
            Err(_) => missing.push(id),
        }
    }

    if !missing.is_empty() {
        genres.extend(
            post::<Vec<docs::IgdbAnnotation>>(
                connection,
                GENRES_ENDPOINT,
                &format!(
                    "fields *; where id = ({});",
                    missing
                        .into_iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<String>>()
                        .join(",")
                ),
            )
            .await?
            .into_iter()
            .map(|genre| genre.name),
        );
    }

    Ok(genres)
}

/// Returns game keywords based on id from the igdb/keywords endpoint.
#[instrument(level = "trace", skip(connection, firestore))]
async fn get_keywords(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    ids: &[u64],
) -> Result<Vec<String>, Status> {
    let mut keywords = vec![];
    let mut missing = vec![];
    for id in ids {
        match firestore::keywords::read(firestore, *id).await {
            Ok(kw) => keywords.push(kw.name),
            Err(_) => missing.push(id),
        }
    }

    if !missing.is_empty() {
        keywords.extend(
            post::<Vec<docs::IgdbAnnotation>>(
                connection,
                KEYWORDS_ENDPOINT,
                &format!(
                    "fields *; where id = ({});",
                    missing
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<String>>()
                        .join(",")
                ),
            )
            .await?
            .into_iter()
            .map(|keyword| keyword.name),
        );
    }

    Ok(keywords)
}

/// Returns game screenshots based on id from the igdb/screenshots endpoint.
#[instrument(level = "trace", skip(connection))]
async fn get_artwork(connection: &IgdbConnection, ids: &[u64]) -> Result<Vec<Image>, Status> {
    Ok(post(
        connection,
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
#[instrument(level = "trace", skip(connection))]
async fn get_screenshots(connection: &IgdbConnection, ids: &[u64]) -> Result<Vec<Image>, Status> {
    Ok(post(
        &connection,
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
#[instrument(level = "trace", skip(connection))]
async fn get_websites(
    connection: &IgdbConnection,
    ids: &[u64],
) -> Result<Vec<docs::IgdbWebsite>, Status> {
    Ok(post(
        &connection,
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
#[instrument(level = "trace", skip(connection, firestore))]
async fn get_collection(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    id: u64,
) -> Result<Option<CollectionDigest>, Status> {
    let collection = firestore::collections::read(firestore, id).await;
    match collection {
        Ok(collection) => Ok(Some(CollectionDigest {
            id: collection.id,
            name: collection.name,
            slug: collection.slug,
            igdb_type: CollectionType::Collection,
        })),
        Err(_) => {
            let result: Vec<docs::IgdbAnnotation> = post(
                &connection,
                COLLECTIONS_ENDPOINT,
                &format!("fields *; where id={id};"),
            )
            .await?;

            match result.into_iter().next() {
                Some(collection) => Ok(Some(CollectionDigest {
                    id: collection.id,
                    name: collection.name,
                    slug: collection.slug,
                    igdb_type: CollectionType::Collection,
                })),
                None => Ok(None),
            }
        }
    }
}

/// Returns game franchices based on id from the igdb/frachises endpoint.
#[instrument(level = "trace", skip(connection, firestore))]
async fn get_franchises(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    ids: &[u64],
) -> Result<Vec<CollectionDigest>, Status> {
    let mut franchises = vec![];
    let mut missing = vec![];
    for id in ids {
        match firestore::franchises::read(firestore, *id).await {
            Ok(franchise) => franchises.push(CollectionDigest {
                id: franchise.id,
                name: franchise.name,
                slug: franchise.slug,
                igdb_type: CollectionType::Franchise,
            }),
            Err(_) => missing.push(id),
        }
    }

    if !missing.is_empty() {
        franchises.extend(
            post::<Vec<docs::IgdbAnnotation>>(
                connection,
                FRANCHISES_ENDPOINT,
                &format!(
                    "fields *; where id = ({});",
                    missing
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<String>>()
                        .join(",")
                ),
            )
            .await?
            .into_iter()
            .map(|c| CollectionDigest {
                id: c.id,
                name: c.name,
                slug: c.slug,
                igdb_type: CollectionType::Franchise,
            }),
        );
    }

    Ok(franchises)
}

fn get_role(involved_company: &IgdbInvolvedCompany) -> CompanyRole {
    match involved_company.developer {
        true => CompanyRole::Developer,
        false => match involved_company.publisher {
            true => CompanyRole::Publisher,
            false => match involved_company.porting {
                true => CompanyRole::Porting,
                false => match involved_company.supporting {
                    true => CompanyRole::Support,
                    false => CompanyRole::Unknown,
                },
            },
        },
    }
}

/// Returns game companies involved in the making of the game.
#[instrument(level = "trace", skip(connection, firestore))]
async fn get_involved_companies(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    ids: &[u64],
) -> Result<Vec<CompanyDigest>, Status> {
    // Collect all involved companies for a game entry.
    let involved_companies: Vec<docs::IgdbInvolvedCompany> = post(
        &connection,
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

    let mut companies = vec![];
    let mut missing = vec![];

    for involved_company in &involved_companies {
        if let Some(id) = involved_company.company {
            match firestore::companies::read(firestore, id).await {
                Ok(igdb_company) => companies.push(CompanyDigest {
                    id: igdb_company.id,
                    name: igdb_company.name,
                    slug: igdb_company.slug,
                    role: get_role(involved_company),
                }),
                _ => missing.push(id),
            }
        }
    }

    if !missing.is_empty() {
        companies.extend(
            post::<Vec<docs::IgdbCompany>>(
                &connection,
                COMPANIES_ENDPOINT,
                &format!(
                    "fields *; where id = ({});",
                    missing
                        .into_iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            )
            .await?
            .into_iter()
            .map(|c| CompanyDigest {
                id: c.id,
                name: c.name,
                slug: c.slug,
                role: match involved_companies.iter().find(|ic| match ic.company {
                    Some(cid) => cid == c.id,
                    None => false,
                }) {
                    Some(ic) => get_role(ic),
                    None => CompanyRole::Unknown,
                },
            }),
        );
    }

    Ok(companies)
}

pub const GAMES_ENDPOINT: &str = "games";
pub const EXTERNAL_GAMES_ENDPOINT: &str = "external_games";
pub const COLLECTIONS_ENDPOINT: &str = "collections";
pub const FRANCHISES_ENDPOINT: &str = "franchises";
pub const COMPANIES_ENDPOINT: &str = "companies";
pub const GENRES_ENDPOINT: &str = "genres";
pub const KEYWORDS_ENDPOINT: &str = "keywords";
const COVERS_ENDPOINT: &str = "covers";
const ARTWORKS_ENDPOINT: &str = "artworks";
const SCREENSHOTS_ENDPOINT: &str = "screenshots";
const WEBSITES_ENDPOINT: &str = "websites";
const INVOLVED_COMPANIES_ENDPOINT: &str = "involved_companies";
