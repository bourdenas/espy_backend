use std::{
    cmp::Ordering,
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    api::{CompanyNormalizer, FirestoreApi, MetacriticApi, SteamDataApi, SteamScrape},
    documents::{
        Collection, CollectionDigest, CollectionType, Company, CompanyDigest, CompanyRole,
        GameCategory, GameDigest, GameEntry, IgdbGame, IgdbInvolvedCompany, SteamData, StoreName,
        Website, WebsiteAuthority,
    },
    library, Status,
};

use async_recursion::async_recursion;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use tracing::{error, instrument, trace_span, warn, Instrument};

use super::{endpoints, request::post, IgdbConnection, IgdbLookup};

/// Returns a GameEntry from IGDB that can build the GameDigest doc.
///
/// Updates Firestore structures with fresh game digest data.
#[instrument(level = "info", skip(connection, firestore, igdb_game))]
pub async fn resolve_game_digest(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    igdb_game: IgdbGame,
) -> Result<GameEntry, Status> {
    let mut game_entry = GameEntry::from(igdb_game);
    let igdb_game = &game_entry.igdb_game;

    let external_games = match library::firestore::external_games::get_external_games(
        firestore,
        game_entry.id,
    )
    .await
    {
        Ok(external_games) => external_games,
        Err(status) => {
            warn!("{status}");
            vec![]
        }
    };

    // Spawn a task to retrieve steam data.
    let steam_handle = match external_games
        .iter()
        .find(|e| matches!(e.store_name, StoreName::Steam))
    {
        Some(steam_external) => {
            let steam_appid = steam_external.store_id.clone();
            Some(tokio::spawn(
                async move {
                    let steam = SteamDataApi::new();
                    steam.retrieve_steam_data(&steam_appid).await
                }
                .instrument(trace_span!("spawn_steam_request")),
            ))
        }
        None => None,
    };

    // Spawn a task to retrieve metacritic score.
    let slug = MetacriticApi::guess_id(&igdb_game.url).to_owned();
    let metacritic_handle = tokio::spawn(
        async move { MetacriticApi::get_score(&slug).await }
            .instrument(trace_span!("spawn_metacritic_request")),
    );

    if let Some(cover) = igdb_game.cover {
        let lookup = IgdbLookup::new(connection);
        game_entry.cover = lookup.get_cover(cover).await?;
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
        for company in companies {
            match company.role {
                CompanyRole::Developer => game_entry.developers.push(company),
                CompanyRole::Publisher => game_entry.publishers.push(company),
                CompanyRole::DevPub => {
                    game_entry.developers.push(company.clone());
                    game_entry.publishers.push(company);
                }
                _ => {}
            }
        }
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

    game_entry.release_date = get_release_timestamp(connection, &igdb_game, &steam_data)
        .await?
        .unwrap_or_default();

    if let Some(steam_data) = steam_data {
        adjust_companies(
            &mut game_entry,
            &steam_data.developers,
            &steam_data.publishers,
        );
        game_entry.add_steam_data(steam_data);
    }

    match library::firestore::genres::read(firestore, game_entry.id).await {
        Ok(genres) => game_entry.espy_genres = genres.espy_genres,
        Err(Status::NotFound(_)) => {
            library::firestore::genres::needs_annotation(firestore, &game_entry).await?
        }
        Err(status) => error!("Genre lookup failed: {status}"),
    }

    match metacritic_handle.await {
        Ok(result) => match result {
            Ok(response) => {
                if let Some(metacritic) = response {
                    game_entry
                        .scores
                        .add_metacritic(metacritic, game_entry.release_date);
                }
            }
            Err(status) => warn!("{status}"),
        },
        Err(status) => warn!("{status}"),
    }

    if game_entry.steam_data.is_none() || game_entry.scores.metacritic.is_none() {
        match library::firestore::wikipedia::read(&firestore, game_entry.id).await {
            Ok(wiki_data) => {
                if game_entry.scores.metacritic.is_none() && wiki_data.score.is_some() {
                    game_entry.scores.add_wikipedia(&wiki_data);
                }
                if game_entry.steam_data.is_none() {
                    adjust_companies(
                        &mut game_entry,
                        &wiki_data.developers,
                        &wiki_data.publishers,
                    );
                }
            }
            Err(Status::NotFound(_)) => {
                // pass: no score found
            }
            Err(status) => {
                error!("Score lookup failed: {status}");
            }
        }
    }

    if let Some(gog_external) = external_games
        .into_iter()
        .find(|e| matches!(e.store_name, StoreName::Gog))
    {
        if let Some(gog_data) = gog_external.gog_data {
            game_entry.add_gog_data(gog_data);
        }
    }

    // TODO: Remove these updates from the critical path.
    update_companies(firestore, &game_entry).await;
    update_collections(firestore, &game_entry).await;

    Ok(game_entry)
}

// Filters out developers and publishers in the GameEntry that are not present
// in the external sources. It ignores external sources if they are empty or if
// they filter out all GameEntry devs / pubs.
fn adjust_companies(
    game_entry: &mut GameEntry,
    external_source_devs: &[String],
    external_source_pubs: &[String],
) {
    let external_source_devs = external_source_devs
        .iter()
        .map(|e| CompanyNormalizer::slug(e))
        .map(|e| e.to_lowercase())
        .collect_vec();
    let external_source_pubs = external_source_pubs
        .iter()
        .map(|e| CompanyNormalizer::slug(e))
        .map(|e| e.to_lowercase())
        .collect_vec();

    let filtered = game_entry
        .developers
        .iter()
        .cloned()
        .filter(|digest| {
            external_source_devs.is_empty()
                || external_source_devs.contains(&digest.slug.to_lowercase())
        })
        .collect_vec();
    if !filtered.is_empty() {
        game_entry.developers = filtered;
    }

    let filtered = game_entry
        .publishers
        .iter()
        .cloned()
        .filter(|digest| {
            external_source_pubs.is_empty()
                || external_source_pubs.contains(&digest.slug.to_lowercase())
        })
        .collect_vec();
    if !filtered.is_empty() {
        game_entry.publishers = filtered;
    }
}

fn extract_steam_appid(url: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"https?:\/\/store\.steampowered\.com\/app\/(?P<appid>\d+)").unwrap();
    }
    RE.captures(url)
        .and_then(|cap| cap.name("appid").map(|appid| appid.as_str().to_owned()))
}

/// Returns a fully resolved GameEntry from IGDB that goes beyond the GameDigest doc.
#[async_recursion]
#[instrument(level = "info", skip(connection, firestore, game_entry))]
pub async fn resolve_game_info(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    game_entry: &mut GameEntry,
) -> Result<(), Status> {
    let igdb_game = &game_entry.igdb_game;

    if !igdb_game.keywords.is_empty() {
        game_entry.keywords = get_keywords(firestore, &igdb_game.keywords).await?;
    }

    let lookup = IgdbLookup::new(connection);
    if igdb_game.websites.len() > 0 {
        if let Ok(websites) = lookup.get_websites(&igdb_game.websites).await {
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

    // It is often the case that external_games collection does not contain the
    // Steam connection. Extract the steam_appid from the url if any.
    if game_entry.steam_data.is_none() {
        let steam_appid = game_entry
            .websites
            .iter()
            .find(|website| matches!(website.authority, WebsiteAuthority::Steam))
            .and_then(|website| extract_steam_appid(&website.url));

        if let Some(steam_appid) = steam_appid {
            let steam = SteamDataApi::new();
            game_entry.add_steam_data(steam.retrieve_steam_data(&steam_appid).await?);
        }
    }

    // NOTE: This is a new immutable borrow for `game_entry` in order for borrow
    // checker to drop the original immutable borrow and allow the mutable
    // borrow needed above to add retrieved steam_data.
    let igdb_game = &game_entry.igdb_game;

    let steam_scrape_handle = match &game_entry.steam_data {
        Some(steam_data) => {
            let steam_appid = steam_data.steam_appid.to_string();
            Some(tokio::spawn(
                async move { SteamScrape::scrape(&steam_appid).await }
                    .instrument(trace_span!("spawn_steam_scrape")),
            ))
        }
        None => None,
    };

    // Skip screenshots if they already exist from steam data.
    if !igdb_game.screenshots.is_empty() && game_entry.steam_data.is_none() {
        if let Ok(screenshots) = lookup.get_screenshots(&igdb_game.screenshots).await {
            game_entry.screenshots = screenshots;
        }
    }
    if !igdb_game.artworks.is_empty() {
        if let Ok(artwork) = lookup.get_artwork(&igdb_game.artworks).await {
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

    if let Some(handle) = steam_scrape_handle {
        match handle.await {
            Ok(result) => match result {
                Ok(steam_scrape_data) => {
                    if let Some(steam_data) = &mut game_entry.steam_data {
                        steam_data.user_tags = steam_scrape_data.user_tags;
                    }
                }
                Err(status) => warn!("{status}"),
            },
            Err(status) => warn!("{status}"),
        }
    }

    Ok(())
}

/// Returns IgdbGames included in the bundle of `bundle_id`.
#[instrument(level = "info", skip(connection))]
async fn get_bundle_games_ids(
    connection: &IgdbConnection,
    bundle_id: u64,
) -> Result<Vec<IgdbGame>, Status> {
    post::<Vec<IgdbGame>>(
        &connection,
        endpoints::GAMES,
        &format!("fields id, name; where bundles = ({bundle_id});"),
    )
    .await
}

#[instrument(level = "trace", skip(connection, firestore))]
async fn get_digest(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    id: u64,
) -> Result<GameDigest, Status> {
    match library::firestore::games::read(firestore, id).await {
        Ok(game_entry) => return Ok(GameDigest::from(game_entry)),
        Err(_) => {}
    }

    let lookup = IgdbLookup::new(connection);
    let igdb_game = lookup.get_game(id).await?;
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
    let result = library::firestore::games::batch_read(firestore, ids).await?;
    let mut digests = result
        .documents
        .into_iter()
        .map(|entry| GameDigest::from(entry))
        .collect_vec();

    let lookup = IgdbLookup::new(connection);
    if !result.not_found.is_empty() {
        let games = lookup.get_games(&result.not_found).await?;
        for igdb_game in games {
            digests.push(GameDigest::from(
                resolve_game_digest(connection, firestore, igdb_game).await?,
            ));
        }
    }
    Ok(digests)
}

/// Returns game keywords from their ids.
#[instrument(level = "trace", skip(firestore))]
async fn get_keywords(firestore: &FirestoreApi, ids: &[u64]) -> Result<Vec<String>, Status> {
    let result = library::firestore::keywords::batch_read(firestore, ids).await?;
    Ok(result.documents.into_iter().map(|kw| kw.name).collect())
}

/// Returns game collection based on id from the igdb/collections endpoint.
#[instrument(level = "trace", skip(connection, firestore))]
async fn get_collections(
    connection: &IgdbConnection,
    firestore: &FirestoreApi,
    ids: &[u64],
) -> Result<Vec<CollectionDigest>, Status> {
    let result = library::firestore::collections::batch_read(firestore, ids).await?;
    let mut collections = result
        .documents
        .into_iter()
        .map(|e| CollectionDigest {
            id: e.id,
            name: e.name,
            slug: e.slug,
            igdb_type: CollectionType::Collection,
        })
        .collect_vec();

    let lookup = IgdbLookup::new(connection);
    if !result.not_found.is_empty() {
        collections.extend(
            lookup
                .get_collections(&result.not_found)
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
    let result = library::firestore::franchises::batch_read(firestore, ids).await?;
    let mut franchises = result
        .documents
        .into_iter()
        .map(|e| CollectionDigest {
            id: e.id,
            name: e.name,
            slug: e.slug,
            igdb_type: CollectionType::Franchise,
        })
        .collect_vec();

    let lookup = IgdbLookup::new(connection);
    if !result.not_found.is_empty() {
        franchises.extend(
            lookup
                .get_franchises(&result.not_found)
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
        true => match involved_company.publisher {
            true => CompanyRole::DevPub,
            false => CompanyRole::Developer,
        },
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
    let lookup = IgdbLookup::new(connection);
    let involved_companies = lookup.get_involved_companies(ids).await?;

    let involved = HashMap::<u64, CompanyRole>::from_iter(
        involved_companies
            .iter()
            .filter(|ic| {
                matches!(
                    get_role(ic),
                    CompanyRole::Developer | CompanyRole::Publisher | CompanyRole::DevPub
                )
            })
            .filter(|ic| ic.company.is_some())
            .map(|ic| (ic.company.unwrap(), get_role(ic))),
    );
    let result = library::firestore::companies::batch_read(
        firestore,
        &involved.keys().cloned().collect_vec(),
    )
    .await?;

    let mut companies = result
        .documents
        .into_iter()
        .map(|company_doc| CompanyDigest {
            id: company_doc.id,
            slug: CompanyNormalizer::slug(&company_doc.name),
            name: company_doc.name,
            role: involved[&company_doc.id],
        })
        .collect_vec();

    if !result.not_found.is_empty() {
        companies.extend(
            lookup
                .get_companies(&result.not_found)
                .await?
                .into_iter()
                .map(|igdb_company| CompanyDigest {
                    id: igdb_company.id,
                    slug: CompanyNormalizer::slug(&igdb_company.name),
                    name: igdb_company.name,
                    role: involved[&igdb_company.id],
                }),
        );
    }

    Ok(companies)
}

/// Returns the most appropriate game release timestamp. Trying to return the
/// date of the earliest full release date.
#[instrument(level = "trace", skip(connection, igdb_game, steam_data))]
async fn get_release_timestamp(
    connection: &IgdbConnection,
    igdb_game: &IgdbGame,
    steam_data: &Option<SteamData>,
) -> Result<Option<i64>, Status> {
    let lookup = IgdbLookup::new(connection);
    let mut release_dates = match igdb_game.release_dates.is_empty() {
        false => lookup.get_release_dates(&igdb_game.release_dates).await?,
        true => vec![],
    };

    // Sort release dates if many and push back "Early Releases" to prefer full
    // releases instead.
    release_dates.sort_by(|a, b| match (&a.status, &b.status) {
        (Some(ast), Some(bst)) => {
            if ast.name == bst.name {
                a.date.cmp(&b.date)
            } else {
                if ast.name == "Early Access" {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
        }
        (Some(ast), None) => {
            if ast.name == "Early Access" {
                Ordering::Greater
            } else {
                a.date.cmp(&b.date)
            }
        }
        (None, Some(bst)) => {
            if bst.name == "Early Access" {
                Ordering::Greater
            } else {
                a.date.cmp(&b.date)
            }
        }
        (None, None) => a.date.cmp(&b.date),
    });

    let mut release_dates = release_dates
        .iter()
        .filter(|release_date| release_date.date > 0);

    let igdb_date = match release_dates.next() {
        Some(release_date) => Some(release_date.date),
        None => igdb_game.first_release_date,
    };
    let steam_date = match steam_data {
        Some(steam_data) => steam_data.release_timestamp(),
        None => None,
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(
        if igdb_date.is_none()
            || !steam_date.is_none()
                && (igdb_date.unwrap_or_default() > (now as i64)
                    || igdb_date.unwrap_or_default() == 0
                    || (igdb_date.unwrap_or_default() > steam_date.unwrap_or_default()))
        {
            steam_date
        } else {
            igdb_date
        },
    )
}

/// Make sure that any companies involved in the game are updated to include it.
#[instrument(level = "info", skip(firestore, game_entry))]
async fn update_companies(firestore: &FirestoreApi, game_entry: &GameEntry) {
    if !game_entry.category.is_main_category() {
        return;
    }

    let companies = HashMap::<u64, &CompanyDigest>::from_iter(
        game_entry
            .developers
            .iter()
            .chain(game_entry.publishers.iter())
            .map(|company_digest| (company_digest.id, company_digest)),
    );

    for (id, company_digest) in companies {
        let company = match library::firestore::companies::read(&firestore, id).await {
            // Update game in company.
            Ok(mut company) => {
                let game_digest = GameDigest::from(game_entry.clone()).compact();
                match company_digest.role {
                    CompanyRole::Developer => update_digest(&mut company.developed, game_digest),
                    CompanyRole::Publisher => update_digest(&mut company.published, game_digest),
                    CompanyRole::DevPub => {
                        update_digest(&mut company.developed, game_digest.clone());
                        update_digest(&mut company.published, game_digest);
                    }
                    _ => {}
                }
                company
            }
            // Company was missing. Create it.
            Err(Status::NotFound(_)) => Company {
                id: company_digest.id,
                name: company_digest.name.clone(),
                slug: company_digest.slug.clone(),
                description: String::default(),
                logo: None,
                developed: match company_digest.role {
                    CompanyRole::Developer | CompanyRole::DevPub => {
                        vec![GameDigest::from(game_entry.clone()).compact()]
                    }
                    _ => vec![],
                },
                published: match company_digest.role {
                    CompanyRole::Publisher | CompanyRole::DevPub => {
                        vec![GameDigest::from(game_entry.clone()).compact()]
                    }
                    _ => vec![],
                },
            },
            Err(status) => {
                warn!("Failed to read company={}: {status}", company_digest.id);
                continue;
            }
        };

        if let Err(status) = library::firestore::companies::write(&firestore, &company).await {
            warn!("Failed to write company={}: {status}", company.id)
        }
    }
}

/// Update collections / franchises in the game with a fresh digest.
#[instrument(level = "info", skip(firestore, game_entry))]
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
        CollectionType::Collection => library::firestore::collections::read(&firestore, id).await,
        CollectionType::Franchise => library::firestore::franchises::read(&firestore, id).await,
        CollectionType::Null => Err(Status::invalid_argument("invalid collection type")),
    }
}

async fn write_collection(
    firestore: &FirestoreApi,
    collection_type: CollectionType,
    collection: &Collection,
) -> Result<(), Status> {
    match collection_type {
        CollectionType::Collection => {
            library::firestore::collections::write(&firestore, &collection).await
        }
        CollectionType::Franchise => {
            library::firestore::franchises::write(&firestore, &collection).await
        }
        CollectionType::Null => Err(Status::invalid_argument("invalid collection type")),
    }
}
