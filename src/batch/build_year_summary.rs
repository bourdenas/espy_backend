use std::{
    cmp::min,
    collections::HashSet,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use clap::Parser;
use espy_backend::{
    api::FirestoreApi,
    documents::*,
    library::firestore::{timeline, year},
    *,
};
use firestore::{path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, TryStreamExt};
use itertools::Itertools;
use tracing::instrument;

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    #[clap(long, default_value = "2023")]
    year: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_game_entries")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    let start = chrono::NaiveDateTime::parse_from_str(
        &format!("{}-01-01 00:00:00", opts.year),
        "%Y-%m-%d %H:%M:%S",
    )?
    .timestamp();
    let end = min(
        chrono::NaiveDateTime::parse_from_str(
            &format!("{}-01-01 00:00:00", opts.year + 1),
            "%Y-%m-%d %H:%M:%S",
        )?
        .timestamp(),
        Utc::now().naive_utc().timestamp(),
    );

    let firestore = Arc::new(api::FirestoreApi::connect().await?);

    let game_entries: BoxStream<FirestoreResult<GameEntry>> = firestore
        .db()
        .fluent()
        .select()
        .from("games")
        .filter(|q| {
            q.for_all([
                q.field(path!(GameEntry::release_date))
                    .greater_than_or_equal(start),
                q.field(path!(GameEntry::release_date)).less_than(end),
            ])
        })
        .order_by([(
            path!(GameEntry::release_date),
            FirestoreQueryDirection::Ascending,
        )])
        .obj()
        .stream_query_with_errors()
        .await?;
    let mut games = game_entries.try_collect::<Vec<GameEntry>>().await?;
    println!("Retrieved {} titles.", games.len());
    games.retain(|game| match game.category {
        GameCategory::Dlc
        | GameCategory::Bundle
        | GameCategory::Episode
        | GameCategory::Version
        | GameCategory::Ignore => false,
        _ => true,
    });
    println!("Retained {} titles.", games.len());

    let notable = timeline::read_notable(&firestore).await?;
    let companies = HashSet::<String>::from_iter(notable.legacy_companies.into_iter());
    let collections = HashSet::<String>::from_iter(notable.collections.into_iter());

    let mut partitions = games.into_iter().into_group_map_by(|game| {
        if game.release_date == 0 {
            match is_hyped_tbd(&game) {
                true => "releases",
                false => "ignore",
            }
        } else if is_early_access(&game) {
            match is_popular_early_access(&game) {
                true => "early_access",
                false => "ignore",
            }
        } else if is_expansion(&game) && game.scores.metacritic.is_none() {
            "expansions"
        } else if is_indie(&game) {
            if game.scores.metacritic.is_some() || is_popular(game) {
                match is_casual(game) {
                    true => "casual",
                    false => "indies",
                }
            } else {
                "ignore"
            }
        } else if game.scores.metacritic.is_some()
            || is_popular(game)
            || is_remaster(game)
            || is_notable(game, &companies, &collections)
            || is_gog_classic(&game)
        {
            match is_casual(game) {
                true => "casual",
                false => "releases",
            }
        } else {
            "ignore"
        }
    });

    for (_, digests) in &mut partitions {
        digests.sort_by(|a, b| match b.scores.espy_score.cmp(&a.scores.espy_score) {
            std::cmp::Ordering::Equal => match b.scores.popularity.cmp(&a.scores.popularity) {
                std::cmp::Ordering::Equal => match b.scores.thumbs.cmp(&a.scores.thumbs) {
                    std::cmp::Ordering::Equal => b.scores.hype.cmp(&a.scores.hype),
                    other => other,
                },
                other => other,
            },
            other => other,
        })
    }

    let review = AnnualReview {
        last_updated: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        releases: partitions
            .remove("releases")
            .unwrap_or_default()
            .into_iter()
            .map(|game| GameDigest::from(game))
            .collect(),
        indies: partitions
            .remove("indies")
            .unwrap_or_default()
            .into_iter()
            .map(|game| GameDigest::from(game))
            .collect(),
        expansions: partitions
            .remove("expansions")
            .unwrap_or_default()
            .into_iter()
            .map(|game| GameDigest::from(game))
            .collect(),
        casual: partitions
            .remove("casual")
            .unwrap_or_default()
            .into_iter()
            .map(|game| GameDigest::from(game))
            .collect(),
        early_access: partitions
            .remove("early_access")
            .unwrap_or_default()
            .into_iter()
            .map(|game| GameDigest::from(game))
            .collect(),
        debug: partitions
            .remove("debug")
            .unwrap_or_default()
            .into_iter()
            .map(|game| GameDigest::from(game))
            .collect(),
    };

    let mut i = 0;
    for game in partitions.remove("ignore").unwrap_or_default().iter() {
        println!("#{i} deleting {}({})", game.name, game.id);
        i += 1;
        library::firestore::games::delete(&firestore, game.id).await?;
    }

    year::write(&firestore, &review, opts.year).await?;

    let serialized = serde_json::to_string(&review)?;
    println!(
        "created annual review for {} size: {}KB",
        opts.year,
        serialized.len() / 1024
    );

    Ok(())
}

fn is_popular(game: &GameEntry) -> bool {
    (game.release_year() > 2011 && game.scores.popularity.unwrap_or_default() >= 10000)
        || (game.release_year() <= 2011 && game.scores.popularity.unwrap_or_default() > 0)
}

fn is_remaster(game: &GameEntry) -> bool {
    match game.category {
        GameCategory::Remake | GameCategory::Remaster => true,
        _ => false,
    }
}

fn is_gog_classic(game: &GameEntry) -> bool {
    game.release_year() < 2000
        && game
            .websites
            .iter()
            .any(|website| matches!(website.authority, WebsiteAuthority::Gog))
}

fn is_notable(
    game: &GameEntry,
    companies: &HashSet<String>,
    collections: &HashSet<String>,
) -> bool {
    game.developers.iter().any(|c| companies.contains(&c.name))
        || game
            .collections
            .iter()
            .any(|c| collections.contains(&c.name))
}

fn is_casual(game: &GameEntry) -> bool {
    game.steam_data
        .as_ref()
        .unwrap_or(&SteamData::default())
        .genres
        .iter()
        .any(|genre| genre.description == "Casual")
}

fn is_hyped_tbd(game: &GameEntry) -> bool {
    game.release_date == 0
        && !matches!(game.status, GameStatus::Cancelled)
        && game.scores.hype.unwrap_or_default() > 1
        && game.scores.thumbs.is_none()
        && !is_casual(&game)
}

fn is_early_access(game: &GameEntry) -> bool {
    game.release_year() > 2018
        && matches!(game.status, GameStatus::EarlyAccess)
        && game.scores.metacritic.is_none()
}

fn is_popular_early_access(game: &GameEntry) -> bool {
    game.scores.popularity.unwrap_or_default() >= 5000
}

fn is_expansion(game: &GameEntry) -> bool {
    matches!(
        game.category,
        GameCategory::Expansion | GameCategory::StandaloneExpansion
    )
}

fn is_indie(game: &GameEntry) -> bool {
    game.release_year() > 2007
        && game
            .espy_genres
            .iter()
            .any(|genre| matches!(genre, EspyGenre::Indie))
}

#[instrument(
    level = "info",
    skip(firestore, igdb),
    fields(event_span = "resolve_event")
)]
async fn refresh_game(
    firestore: Arc<FirestoreApi>,
    id: u64,
    igdb: &api::IgdbApi,
) -> Result<(), Status> {
    let igdb_game = igdb.get(id).await?;
    igdb.resolve(firestore, igdb_game).await?;

    Ok(())
}
