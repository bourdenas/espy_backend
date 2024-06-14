use std::cmp::Ordering;

use crate::{
    api::{FirestoreApi, MetacriticApi, SteamDataApi, SteamScrape},
    documents::{
        Collection, CollectionDigest, CollectionType, Company, CompanyDigest, CompanyRole,
        GameCategory, GameDigest, GameEntry, Image, SteamData, Website, WebsiteAuthority,
    },
    library::firestore,
    Status,
};
use async_recursion::async_recursion;
use chrono::NaiveDateTime;
use itertools::Itertools;
use tracing::{error, instrument, trace_span, warn, Instrument};

use super::{
    backend::post,
    docs::{self, IgdbInvolvedCompany},
    IgdbConnection, IgdbGame,
};

/// Returns a GameEntry from IGDB that can build the GameDigest doc.
///
/// Updates Firestore structures with fresh game digest data.
#[instrument(
    level = "trace",
    skip(connection, firestore, igdb_game)
    fields(
        game_id = %igdb_game.id,
        game_name = %igdb_game.name,
    )
)]
pub async fn resolve_game_digest(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    igdb_game: IgdbGame,
) -> Result<GameEntry, Status> {
    let mut game_entry = GameEntry::from(igdb_game);
    let igdb_game = &game_entry.igdb_game;

    // Spawn a task to retrieve steam data.
    let steam_handle = match firestore::external_games::get_steam_id(firestore, game_entry.id).await
    {
        Ok(steam_appid) => Some(tokio::spawn(
            async move {
                let steam = SteamDataApi::new();
                steam.retrieve_steam_data(&steam_appid).await
            }
            .instrument(trace_span!("spawn_steam_request")),
        )),
        Err(status) => {
            warn!("{status}");
            None
        }
    };

    // Spawn a task to retrieve metacritic score.
    let slug = MetacriticApi::guess_id(&igdb_game.url).to_owned();
    let metacritic_handle = tokio::spawn(
        async move { MetacriticApi::get_score(&slug).await }
            .instrument(trace_span!("spawn_metacritic_request")),
    );

    if let Some(cover) = igdb_game.cover {
        game_entry.cover = get_cover(connection, cover).await?;
    }

    let mut collections = [
        match igdb_game.collection {
            Some(id) => vec![id],
            None => vec![],
        },
        igdb_game.collections.clone(),
    ]
    .concat();
    if !collections.is_empty() {
        collections.sort();
        collections.dedup();
        game_entry
            .collections
            .extend(get_collections(connection, firestore, &collections).await?);
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
            .extend(get_franchises(connection, firestore, &franchises).await?);
    }

    if !igdb_game.involved_companies.is_empty() {
        let companies =
            get_involved_companies(connection, firestore, &igdb_game.involved_companies).await?;
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

    let mut steam_data = None;
    if let Some(handle) = steam_handle {
        match handle.await {
            Ok(result) => match result {
                Ok(data) => steam_data = Some(data),
                Err(status) => warn!("{status}"),
            },
            Err(status) => warn!("{status}"),
        }
    }

    if !igdb_game.release_dates.is_empty() {
        game_entry.release_date = get_release_date(connection, &igdb_game, &steam_data)
            .await?
            .unwrap_or_default();
    }

    if let Some(steam_data) = steam_data {
        game_entry.add_steam_data(steam_data);
    }
    game_entry.resolve_genres();

    match firestore::genres::read(firestore, game_entry.id).await? {
        Some(genres) => game_entry.espy_genres = genres.espy_genres,
        None => firestore::genres::needs_annotation(firestore, &game_entry).await?,
    }

    match metacritic_handle.await {
        Ok(response) => {
            if let Some(metacritic) = response {
                game_entry
                    .scores
                    .add_metacritic(metacritic, game_entry.release_date);
            }
        }
        Err(status) => warn!("{status}"),
    }

    if game_entry.scores.metacritic.is_none() {
        match firestore::scores::read(&firestore, game_entry.id).await {
            Ok(lookup) => {
                let scores = &mut game_entry.scores;
                scores.metacritic = lookup.scores.metacritic;
                scores.metacritic_source = lookup.scores.metacritic_source;
                scores.espy_score = lookup.scores.espy_score;
            }
            Err(Status::NotFound(_)) => {
                // pass: no score found
            }
            Err(status) => {
                error!("Score lookup failed: {status}");
            }
        }
    }

    // TODO: Remove these updates from the critical path.
    update_companies(firestore, &game_entry).await;
    update_collections(firestore, &game_entry).await;

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
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    game_entry: &mut GameEntry,
) -> Result<(), Status> {
    let igdb_game = &game_entry.igdb_game;

    let steam_handle = match &game_entry.steam_data {
        Some(steam_data) => {
            let website = format!(
                "https://store.steampowered.com/app/{}/",
                steam_data.steam_appid
            );
            Some(tokio::spawn(
                async move { SteamScrape::scrape(&website).await }
                    .instrument(trace_span!("spawn_steam_scrape")),
            ))
        }
        None => None,
    };

    if !igdb_game.keywords.is_empty() {
        game_entry.keywords = get_keywords(firestore, &igdb_game.keywords).await?;
    }

    if igdb_game.websites.len() > 0 {
        if let Ok(websites) = get_websites(connection, &igdb_game.websites).await {
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

    // Skip screenshots if they already exist from steam data.
    if !igdb_game.screenshots.is_empty() && game_entry.steam_data.is_none() {
        if let Ok(screenshots) = get_screenshots(connection, &igdb_game.screenshots).await {
            game_entry.screenshots = screenshots;
        }
    }
    if !igdb_game.artworks.is_empty() {
        if let Ok(artwork) = get_artwork(connection, &igdb_game.artworks).await {
            game_entry.artwork = artwork;
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
        if let Ok(game) = get_digest(connection, firestore, id).await {
            game_entry.parent = Some(game);
        };
    }
    if !igdb_game.expansions.is_empty() {
        if let Ok(digests) = get_digests(connection, firestore, &igdb_game.expansions).await {
            game_entry.expansions = digests;
        }
    }
    if !igdb_game.standalone_expansions.is_empty() {
        if let Ok(mut digests) =
            get_digests(connection, firestore, &igdb_game.standalone_expansions).await
        {
            game_entry.expansions.append(&mut digests);
        }
    }
    if !igdb_game.dlcs.is_empty() {
        if let Ok(digests) = get_digests(connection, firestore, &igdb_game.dlcs).await {
            game_entry.dlcs = digests;
        }
    }
    if !igdb_game.remakes.is_empty() {
        if let Ok(digests) = get_digests(connection, firestore, &igdb_game.remakes).await {
            game_entry.remakes = digests;
        }
    }
    if !igdb_game.remasters.is_empty() {
        if let Ok(digests) = get_digests(connection, firestore, &igdb_game.remasters).await {
            game_entry.remasters = digests;
        }
    }
    if matches!(
        game_entry.category,
        GameCategory::Bundle | GameCategory::Version
    ) {
        let game_ids = get_bundle_games_ids(connection, game_entry.id)
            .await?
            .into_iter()
            .map(|e| e.id)
            .collect_vec();

        if let Ok(digests) = get_digests(connection, firestore, &game_ids).await {
            game_entry.contents = digests;
        }
    }

    if let Some(handle) = steam_handle {
        match handle.await {
            Ok(result) => {
                if let Some(steam_scrape_data) = result {
                    if let Some(steam_data) = &mut game_entry.steam_data {
                        steam_data.user_tags = steam_scrape_data.user_tags;
                    }
                }
            }
            Err(status) => warn!("{status}"),
        }
    }

    Ok(())
}

/// Returns IgdbGames included in the bundle of `bundle_id`.
#[instrument(level = "trace", skip(connection))]
async fn get_bundle_games_ids(
    connection: &IgdbConnection,
    bundle_id: u64,
) -> Result<Vec<IgdbGame>, Status> {
    post::<Vec<IgdbGame>>(
        &connection,
        GAMES_ENDPOINT,
        &format!("fields id, name; where bundles = ({bundle_id});"),
    )
    .await
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

#[instrument(level = "trace", skip(connection, firestore))]
async fn get_digest(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    id: u64,
) -> Result<GameDigest, Status> {
    match firestore::games::read(firestore, id).await {
        Ok(game_entry) => return Ok(GameDigest::from(game_entry)),
        Err(_) => {}
    }

    let igdb_game = get_game(connection, id).await?;
    Ok(GameDigest::from(
        resolve_game_digest(connection, firestore, igdb_game).await?,
    ))
}

#[instrument(level = "trace", skip(connection, firestore))]
async fn get_digests(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    ids: &[u64],
) -> Result<Vec<GameDigest>, Status> {
    let (game_entries, missing) = firestore::games::batch_read(firestore, ids).await?;
    let mut digests = game_entries
        .into_iter()
        .map(|entry| GameDigest::from(entry))
        .collect_vec();

    if !missing.is_empty() {
        let games = get_games(connection, &missing).await?;
        for igdb_game in games {
            digests.push(GameDigest::from(
                resolve_game_digest(connection, firestore, igdb_game).await?,
            ));
        }
    }
    Ok(digests)
}

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

#[instrument(level = "trace", skip(connection))]
async fn get_games(connection: &IgdbConnection, ids: &[u64]) -> Result<Vec<IgdbGame>, Status> {
    post::<Vec<IgdbGame>>(
        connection,
        GAMES_ENDPOINT,
        &format!(
            "fields *; where id = ({});",
            ids.into_iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
    )
    .await
}

/// Returns game keywords from their ids.
#[instrument(level = "trace", skip(firestore))]
async fn get_keywords(firestore: &FirestoreApi, ids: &[u64]) -> Result<Vec<String>, Status> {
    Ok(firestore::keywords::batch_read(firestore, ids)
        .await?
        .iter()
        .map(|kw| kw.name.clone())
        .collect())
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
async fn get_collections(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    ids: &[u64],
) -> Result<Vec<CollectionDigest>, Status> {
    let (collections, missing) = firestore::collections::batch_read(firestore, ids).await?;
    let mut collections = collections
        .into_iter()
        .map(|e| CollectionDigest {
            id: e.id,
            name: e.name,
            slug: e.slug,
            igdb_type: CollectionType::Collection,
        })
        .collect_vec();

    if !missing.is_empty() {
        collections.extend(
            post::<Vec<docs::IgdbAnnotation>>(
                connection,
                COLLECTIONS_ENDPOINT,
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
            .map(|e| CollectionDigest {
                id: e.id,
                name: e.name,
                slug: e.slug,
                igdb_type: CollectionType::Collection,
            }),
        );
    }

    Ok(collections)
}

/// Returns game franchices based on id from the igdb/frachises endpoint.
#[instrument(level = "trace", skip(connection, firestore))]
async fn get_franchises(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    ids: &[u64],
) -> Result<Vec<CollectionDigest>, Status> {
    let (franchises, missing) = firestore::franchises::batch_read(firestore, ids).await?;
    let mut franchises = franchises
        .into_iter()
        .map(|e| CollectionDigest {
            id: e.id,
            name: e.name,
            slug: e.slug,
            igdb_type: CollectionType::Franchise,
        })
        .collect_vec();

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
            .map(|e| CollectionDigest {
                id: e.id,
                name: e.name,
                slug: e.slug,
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

/// Returns the most appropriate game release date. Trying to retun the date of
/// the PC full release.
#[instrument(level = "trace", skip(connection, igdb_game, steam_data))]
async fn get_release_date(
    connection: &IgdbConnection,
    igdb_game: &IgdbGame,
    steam_data: &Option<SteamData>,
) -> Result<Option<i64>, Status> {
    let mut release_dates = post::<Vec<docs::ReleaseDate>>(
        connection,
        RELEASE_DATES_ENDPOINT,
        &format!(
            "fields category, date, status.name; where id = ({});",
            igdb_game
                .release_dates
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",")
        ),
    )
    .await?;

    // Sort release dates if many to bring the earliest "Full Release" first.
    release_dates.sort_by(|a, b| match (&a.status, &b.status) {
        (Some(ast), Some(bst)) => {
            if ast.name == bst.name {
                a.date.cmp(&b.date)
            } else {
                if ast.name == "Full Release" {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        }
        (Some(ast), None) => {
            if ast.name == "Full Release" {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (None, Some(bst)) => {
            if bst.name == "Full Release" {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (None, None) => a.date.cmp(&b.date),
    });

    let mut release_dates = release_dates
        .iter()
        .filter(|release_date| release_date.date > 0);

    // If IGDB date is exact (category == 0) or release is before 2008 then
    // prefer the IGDB release date. Steam's release dates refer to the release
    // on the platform instead of the game itself. For games before 2008 this is
    // problematic.
    if let Some(release_date) = release_dates.next() {
        if release_date.category == 0 || release_date.date < Y2008 {
            Ok(Some(release_date.date))
        } else {
            if let Some(steam_data) = steam_data {
                if let Some(date) = &steam_data.release_date {
                    let date = match NaiveDateTime::parse_from_str(
                        &format!("{} 12:00:00", &date.date),
                        "%b %e, %Y %H:%M:%S",
                    ) {
                        Ok(date) => Some(date.timestamp()),
                        Err(_) => match NaiveDateTime::parse_from_str(
                            &format!("{} 12:00:00", &date.date),
                            "%e %b, %Y %H:%M:%S",
                        ) {
                            Ok(date) => Some(date.timestamp()),
                            Err(_status) => Some(release_date.date),
                        },
                    };
                    return Ok(date);
                }
            }
            Ok(Some(release_date.date))
        }
    } else {
        Ok(igdb_game.first_release_date)
    }
}

const Y2008: i64 = 1199142000;

/// Make sure that any companies involved in the game are updated to include it.
#[instrument(level = "trace", skip(firestore, game_entry))]
async fn update_companies(firestore: &FirestoreApi, game_entry: &GameEntry) {
    for (companies, company_role) in [
        (&game_entry.developers, CompanyRole::Developer),
        (&game_entry.publishers, CompanyRole::Publisher),
    ] {
        for company in companies {
            let company = match firestore::companies::read(&firestore, company.id).await {
                // Update game in company.
                Ok(mut company) => {
                    update_digest(
                        match company_role {
                            CompanyRole::Developer => &mut company.developed,
                            CompanyRole::Publisher => &mut company.published,
                            _ => panic!("Unexpected company role"),
                        },
                        GameDigest::from(game_entry.clone()),
                    );
                    company
                }
                // Company was missing.
                Err(Status::NotFound(_)) => Company {
                    id: company.id,
                    name: company.name.clone(),
                    slug: company.slug.clone(),
                    developed: match company_role {
                        CompanyRole::Developer => vec![GameDigest::from(game_entry.clone())],
                        _ => vec![],
                    },
                    published: match company_role {
                        CompanyRole::Publisher => vec![GameDigest::from(game_entry.clone())],
                        _ => vec![],
                    },
                    ..Default::default()
                },
                Err(status) => {
                    warn!("Failed to read company={}: {status}", company.id);
                    continue;
                }
            };

            if let Err(status) = firestore::companies::write(&firestore, &company).await {
                warn!("Failed to write company={}: {status}", company.id)
            }
        }
    }
}

/// Update collections / franchises in the game with a fresh digest.
#[instrument(level = "trace", skip(firestore, game_entry))]
async fn update_collections(firestore: &FirestoreApi, game_entry: &GameEntry) {
    for (collections, collection_type) in [
        (&game_entry.collections, CollectionType::Collection),
        (&game_entry.franchises, CollectionType::Franchise),
    ] {
        for collection in collections {
            let collection = match read_collection(&firestore, collection_type, collection.id).await
            {
                Ok(mut collection) => {
                    update_digest(&mut collection.games, GameDigest::from(game_entry.clone()));
                    collection
                }
                Err(Status::NotFound(_)) => {
                    // Collection was missing.
                    Collection {
                        id: collection.id,
                        name: collection.name.clone(),
                        slug: collection.slug.clone(),
                        games: vec![GameDigest::from(game_entry.clone())],
                        ..Default::default()
                    }
                }
                Err(status) => {
                    warn!("Failed to read collection={}: {status}", collection.id);
                    continue;
                }
            };

            if let Err(status) = write_collection(&firestore, collection_type, &collection).await {
                warn!("Failed to write collection={}: {status}", collection.id);
            }
        }
    }
}

fn update_digest(digests: &mut Vec<GameDigest>, digest: GameDigest) {
    match digests.iter_mut().find(|game| game.id == digest.id) {
        // Update game in collection.
        Some(game) => *game = digest,
        // Game was missing from the collection.
        None => digests.push(digest),
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

pub const GAMES_ENDPOINT: &str = "games";
pub const EXTERNAL_GAMES_ENDPOINT: &str = "external_games";
pub const COLLECTIONS_ENDPOINT: &str = "collections";
pub const FRANCHISES_ENDPOINT: &str = "franchises";
pub const COMPANIES_ENDPOINT: &str = "companies";
pub const GENRES_ENDPOINT: &str = "genres";
pub const KEYWORDS_ENDPOINT: &str = "keywords";
const RELEASE_DATES_ENDPOINT: &str = "release_dates";
const COVERS_ENDPOINT: &str = "covers";
const ARTWORKS_ENDPOINT: &str = "artworks";
const SCREENSHOTS_ENDPOINT: &str = "screenshots";
const WEBSITES_ENDPOINT: &str = "websites";
const INVOLVED_COMPANIES_ENDPOINT: &str = "involved_companies";
