use clap::Parser;
use espy_backend::{
    documents::{GameCategory, GameDigest, GameEntry, Timeline},
    library::firestore::timeline,
    Status, Tracing,
};
use firestore::{path, FirestoreDb, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, TryStreamExt};
use itertools::Itertools;
use std::{
    collections::HashSet,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::info;

#[derive(Parser)]
struct Opts {
    #[clap(long)]
    prod_tracing: bool,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    let opts: Opts = Opts::parse();

    match opts.prod_tracing {
        false => Tracing::setup("build-timeline")?,
        true => Tracing::setup_prod("build-timeline")?,
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let recent_past = SystemTime::now()
        .checked_sub(Duration::from_secs(6 * 30 * 24 * 60 * 60))
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let db = FirestoreDb::new("espy-library").await?;

    let notable = timeline::read_notable(&db).await?;
    let notable = HashSet::<String>::from_iter(notable.companies.into_iter());

    let upcoming: BoxStream<FirestoreResult<GameEntry>> = db
        .fluent()
        .select()
        .from("games")
        .filter(|q| {
            q.for_all([q
                .field(path!(GameEntry::release_date))
                .greater_than_or_equal(now)])
        })
        .order_by([(
            path!(GameEntry::release_date),
            FirestoreQueryDirection::Ascending,
        )])
        .obj()
        .stream_query_with_errors()
        .await?;
    let upcoming = upcoming.try_collect::<Vec<GameEntry>>().await?;
    info!("upcoming = {}", upcoming.len());

    let upcoming = upcoming
        .into_iter()
        .filter(|entry| match entry.category {
            GameCategory::Main
            | GameCategory::Dlc
            | GameCategory::Expansion
            | GameCategory::StandaloneExpansion
            | GameCategory::Remake
            | GameCategory::Remaster => true,
            _ => false,
        })
        .filter(|entry| {
            entry.popularity.unwrap_or_default() > UPCOMING_POPULARITY_THRESHOLD
                || entry
                    .developers
                    .iter()
                    .any(|dev| notable.contains(&dev.name))
                || entry
                    .publishers
                    .iter()
                    .any(|publ| notable.contains(&publ.name))
        })
        .collect_vec();
    info!("upcoming after filtering = {}", upcoming.len());

    let recent: BoxStream<FirestoreResult<GameEntry>> = db
        .fluent()
        .select()
        .from("games")
        .filter(|q| {
            q.for_all([
                q.field(path!(GameEntry::release_date))
                    .less_than_or_equal(now),
                q.field(path!(GameEntry::release_date))
                    .greater_than_or_equal(recent_past),
            ])
        })
        .order_by([(
            path!(GameEntry::release_date),
            FirestoreQueryDirection::Descending,
        )])
        .obj()
        .stream_query_with_errors()
        .await?;
    let recent = recent.try_collect::<Vec<GameEntry>>().await?;
    info!("recent = {}", recent.len());

    // TODO: IGDB resolve recently released games (e.g. 2-3 days)
    // TODO: Steam fetch last week's releases (e.g. 7 days)
    // TODO: Update recent with new GameEntry versions.

    let recent = recent
        .into_iter()
        .filter(|entry| match entry.category {
            GameCategory::Main
            | GameCategory::Dlc
            | GameCategory::Expansion
            | GameCategory::StandaloneExpansion
            | GameCategory::Remake
            | GameCategory::Remaster => true,
            _ => false,
        })
        .filter(|entry| match entry.popularity {
            Some(value) => match entry.category {
                GameCategory::Main => value >= RECENT_POPULARITY_THRESHOLD,
                _ => value >= RECENT_POPULARITY_THRESHOLD_DLC,
            },
            None => {
                entry
                    .developers
                    .iter()
                    .any(|dev| notable.contains(&dev.name))
                    || entry
                        .publishers
                        .iter()
                        .any(|publ| notable.contains(&publ.name))
            }
        })
        .collect_vec();
    info!("recent after filtering = {}", recent.len());

    let timeline = Timeline {
        last_updated: now,
        upcoming: upcoming
            .iter()
            .map(|game_entry| GameDigest::from(game_entry.clone()))
            .collect(),
        recent: recent
            .iter()
            .map(|game_entry| GameDigest::from(game_entry.clone()))
            .collect(),
    };

    timeline::write(&db, &timeline).await?;

    let serialized = serde_json::to_string(&timeline)?;
    info!("create frontpage size: {}KB", serialized.len() / 1024);

    Ok(())
}

const UPCOMING_POPULARITY_THRESHOLD: u64 = 1;
const RECENT_POPULARITY_THRESHOLD: u64 = 500;
const RECENT_POPULARITY_THRESHOLD_DLC: u64 = 100;
