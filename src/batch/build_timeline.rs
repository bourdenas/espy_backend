use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{Datelike, NaiveDateTime, Utc};
use clap::Parser;
use espy_backend::{
    api::{self, FirestoreApi},
    documents::{
        Frontpage, GameCategory, GameDigest, GameEntry, GameStatus, ReleaseEvent, Timeline,
    },
    library::firestore::{frontpage, notable, timeline},
    util, Status, Tracing,
};
use firestore::{path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, TryStreamExt};
use itertools::Itertools;
use tracing::{error, info};

#[derive(Parser)]
struct Opts {
    #[clap(long)]
    prod_tracing: bool,

    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "false")]
    skip_update: bool,
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
        .checked_sub(Duration::from_secs(25 * MONTH_IN_SECONDS))
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let firestore = FirestoreApi::connect().await?;

    let notable = notable::read(&firestore).await?;
    let notable = HashSet::<String>::from_iter(notable.companies.into_iter());

    let upcoming: BoxStream<FirestoreResult<GameEntry>> = firestore
        .db()
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
            FirestoreQueryDirection::Descending,
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
            | GameCategory::Expansion
            | GameCategory::StandaloneExpansion
            | GameCategory::Remake
            | GameCategory::Remaster => true,
            _ => false,
        })
        .filter(|entry| {
            entry.scores.hype.unwrap_or_default() > UPCOMING_HYPE_THRESHOLD
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

    let recent: BoxStream<FirestoreResult<GameEntry>> = firestore
        .db()
        .fluent()
        .select()
        .from("games")
        .filter(|q| {
            q.for_all([
                q.field(path!(GameEntry::release_date)).less_than(now),
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
    let mut recent = recent.try_collect::<Vec<GameEntry>>().await?;
    info!("recent = {}", recent.len());

    if !opts.skip_update {
        if let Err(status) = update_recent(&opts.key_store, &mut recent).await {
            error!("Failed to update GameEntries: {status}");
        }
    }

    let recent = recent
        .into_iter()
        .filter(|entry| match entry.category {
            GameCategory::Main
            | GameCategory::Expansion
            | GameCategory::StandaloneExpansion
            | GameCategory::Remake
            | GameCategory::Remaster => true,
            _ => false,
        })
        .filter(|entry| {
            entry.scores.hype.unwrap_or_default() > UPCOMING_HYPE_THRESHOLD
                || entry.scores.metacritic.is_some()
                || entry
                    .developers
                    .iter()
                    .any(|dev| notable.contains(&dev.name))
                || entry
                    .publishers
                    .iter()
                    .any(|publ| notable.contains(&publ.name))
                || match entry.status {
                    GameStatus::EarlyAccess => {
                        entry.scores.popularity.unwrap_or_default()
                            > EARLY_ACCESS_POPULARITY_THRESHOLD
                    }
                    _ => false,
                }
        })
        .collect_vec();
    info!("recent after filtering = {}", recent.len());

    build_frontpage(&firestore, &upcoming, &recent).await?;
    build_timeline(&firestore, &upcoming, &recent).await?;

    Ok(())
}

async fn build_frontpage(
    firestore: &FirestoreApi,
    future: &[GameEntry],
    past: &[GameEntry],
) -> Result<(), Status> {
    let today = Utc::now().naive_utc();

    let games = future.iter().chain(past.iter()).filter(|game_entry| {
        let release_date = NaiveDateTime::from_timestamp_opt(game_entry.release_date, 0).unwrap();
        let diff = today.signed_duration_since(release_date);
        diff.num_days().abs() <= 30
    });

    let release_group = |entry: &GameEntry| -> (String, String) {
        let release_date = NaiveDateTime::from_timestamp_opt(entry.release_date, 0).unwrap();
        (
            release_date.format("%-d %b").to_string(),
            release_date.format("%Y").to_string(),
        )
    };

    let releases = games
        .into_iter()
        .group_by(|entry| release_group(entry))
        .into_iter()
        .map(|(key, games)| {
            let mut games = games
                .map(|game| GameDigest::from(game.clone()))
                .collect_vec();
            games.sort_by(|a, b| b.scores.cmp(&a.scores));
            ReleaseEvent {
                label: key.0,
                year: key.1,
                games,
            }
        })
        .collect_vec();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let frontpage = Frontpage {
        last_updated: now,
        releases,
        today: vec![],
        recent: vec![],
        upcoming: vec![],
        new: vec![],
        hyped: vec![],
    };

    frontpage::write(&firestore, &frontpage).await?;

    let serialized = serde_json::to_string(&frontpage)?;
    info!("created timeline size: {}KB", serialized.len() / 1024);

    Ok(())
}

async fn build_timeline(
    firestore: &FirestoreApi,
    future: &[GameEntry],
    past: &[GameEntry],
) -> Result<(), Status> {
    let today = Utc::now().naive_utc();
    let release_group = |entry: &GameEntry| -> (String, String) {
        let release_date = NaiveDateTime::from_timestamp_opt(entry.release_date, 0).unwrap();
        let diff = today.signed_duration_since(release_date);
        let is_future = diff.num_days() < 0;

        let label = if diff.num_days().abs() <= 7 {
            release_date.format("%-d %b").to_string()
        } else if is_future && release_date.month() == 12 && release_date.day() == 31 {
            release_date.year().to_string()
        } else if is_future && release_date.month() == 9 && release_date.day() == 30 {
            "Q3".to_owned()
        } else if is_future && release_date.month() == 6 && release_date.day() == 30 {
            "Q2".to_owned()
        } else if is_future && release_date.month() == 3 && release_date.day() == 31 {
            "Q1".to_owned()
        } else {
            release_date.format("%b").to_string()
        };

        (label, release_date.format("%Y").to_string())
    };

    let mut releases = future
        .into_iter()
        .group_by(|entry| release_group(&entry))
        .into_iter()
        .map(|(key, games)| {
            let mut games = games
                .map(|game| GameDigest::from(game.clone()))
                .collect_vec();
            games.sort_by(|a, b| b.scores.hype.cmp(&a.scores.hype));
            ReleaseEvent {
                label: key.0,
                year: key.1,
                games,
            }
        })
        .collect_vec();

    releases.extend(
        past.into_iter()
            .group_by(|entry| release_group(&entry))
            .into_iter()
            .map(|(key, games)| {
                let mut games = games
                    .map(|game| GameDigest::from(game.clone()))
                    .collect_vec();
                games.sort_by(|a, b| b.scores.espy_score.cmp(&a.scores.espy_score));
                ReleaseEvent {
                    label: key.0,
                    year: key.1,
                    games,
                }
            }),
    );

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let timeline = Timeline {
        last_updated: now,
        releases,
    };

    timeline::write(&firestore, &timeline).await?;

    let serialized = serde_json::to_string(&timeline)?;
    info!("created timeline size: {}KB", serialized.len() / 1024);

    Ok(())
}

async fn update_recent(keys_path: &str, recent: &mut [GameEntry]) -> Result<(), Status> {
    let d7 = SystemTime::now()
        .checked_sub(Duration::from_secs(7 * DAY_IN_SECONDS))
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let keys = util::keys::Keys::from_file(keys_path).unwrap();
    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    let firestore = Arc::new(FirestoreApi::connect().await?);
    for game in recent {
        if game.release_date as u64 >= d7 {
            info!("Updating '{}'...", game.name);
            match igdb.get(game.id).await {
                Ok(igdb_game) => match igdb.resolve(Arc::clone(&firestore), igdb_game).await {
                    Ok(update) => *game = update,
                    Err(e) => error!("{e}"),
                },
                Err(e) => error!("{e}"),
            }
        } else {
            break;
        }
    }

    Ok(())
}

const DAY_IN_SECONDS: u64 = 24 * 60 * 60;
const MONTH_IN_SECONDS: u64 = 30 * 24 * 60 * 60;

const UPCOMING_HYPE_THRESHOLD: u64 = 1;
const EARLY_ACCESS_POPULARITY_THRESHOLD: u64 = 5000;
