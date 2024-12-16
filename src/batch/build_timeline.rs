use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Datelike, Utc};
use clap::Parser;
use espy_backend::{
    api::FirestoreApi,
    documents::{
        Frontpage, GameCategory, GameDigest, GameEntry, GameStatus, ReleaseEvent, Timeline,
    },
    library::{
        self,
        firestore::{frontpage, notable, timeline},
    },
    resolver::ResolveApi,
    Status, Tracing,
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

    /// URL of the resolver backend.
    #[clap(
        long,
        default_value = "https://resolver-478783154654.europe-west1.run.app"
    )]
    resolver_backend: String,

    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "false")]
    skip_update: bool,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    let opts: Opts = Opts::parse();

    match opts.prod_tracing {
        false => Tracing::setup("build-timeline")?,
        true => Tracing::setup_prod("build-timeline", "timeline_logs")?,
    }

    let firestore = FirestoreApi::connect().await?;

    let notable = notable::read(&firestore).await?;
    let notable = HashSet::<String>::from_iter(notable.companies.into_iter());

    let undated = get_undated(&firestore, &notable).await?;
    let upcoming = get_upcoming(&firestore, &notable).await?;
    let recent = get_recent(&firestore, &notable).await?;

    let mut dated = upcoming.into_iter().chain(recent.into_iter()).collect_vec();

    if !opts.skip_update {
        let now = Utc::now();
        let closeby = dated
            .iter_mut()
            .filter(|game_entry| {
                let diff = DateTime::from_timestamp(game_entry.release_date, 0)
                    .unwrap()
                    .signed_duration_since(now);
                diff.num_days().abs() <= 7
            })
            .collect_vec();
        if let Err(status) = update(opts.resolver_backend, closeby).await {
            error!("Failed to update GameEntries: {status}");
        }
    }

    build_frontpage(&firestore, &dated).await?;
    build_timeline(&firestore, &dated, &undated).await?;

    Ok(())
}

async fn build_frontpage(firestore: &FirestoreApi, entries: &[GameEntry]) -> Result<(), Status> {
    let today = Utc::now();

    let games = entries.iter().filter(|game_entry| {
        let diff = DateTime::from_timestamp(game_entry.release_date, 0)
            .unwrap()
            .signed_duration_since(today);
        diff.num_days().abs() <= 30
    });

    // Returns a tuple of two strings that represent date and year of the
    // GameEntry's release.
    let release_group = |entry: &GameEntry| -> (String, String) {
        let release_date = DateTime::from_timestamp(entry.release_date, 0).unwrap();
        (
            release_date.format("%-d %b").to_string(),
            release_date.format("%Y").to_string(),
        )
    };

    let timeline = games
        .into_iter()
        .chunk_by(|entry| release_group(entry))
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

    let today_label = today.format("%-d %b").to_string();
    let today_releases = match timeline.iter().find(|event| event.label == today_label) {
        Some(event) => event.games.clone(),
        None => vec![],
    };

    let upcoming_releases = entries
        .iter()
        .filter(|game_entry| {
            let diff = DateTime::from_timestamp(game_entry.release_date, 0)
                .unwrap()
                .signed_duration_since(today);
            diff.num_days() > 0 && diff.num_days() <= 30
        })
        .map(|game| GameDigest::from(game.clone()))
        .sorted_by(|a, b| b.scores.cmp(&a.scores))
        .collect();

    let recent_releases = entries
        .iter()
        .filter(|game_entry| {
            let diff = DateTime::from_timestamp(game_entry.release_date, 0)
                .unwrap()
                .signed_duration_since(today);
            diff.num_days() < 0 && diff.num_days() >= -30
        })
        .filter(|game| {
            game.scores.metacritic.is_some() || game.scores.popularity.unwrap_or_default() > 1000
        })
        .map(|game| GameDigest::from(game.clone()))
        .sorted_by(|a, b| b.scores.cmp(&a.scores))
        .collect();

    let hyped = entries
        .iter()
        .filter(|game_entry| {
            let diff = DateTime::from_timestamp(game_entry.release_date, 0)
                .unwrap()
                .signed_duration_since(today);
            diff.num_days() > 30 && game_entry.has_release_date()
        })
        .sorted_by(|a, b| b.scores.hype.cmp(&a.scores.hype))
        .take(20)
        .map(|game| GameDigest::from(game.clone()))
        .collect();

    let frontpage = Frontpage {
        last_updated: now,
        timeline,
        today_releases,
        upcoming_releases,
        recent_releases,
        recent_release_dates: vec![],
        recent_announcements: vec![],
        hyped,
    };

    frontpage::write(&firestore, &frontpage).await?;

    let serialized = serde_json::to_string(&frontpage)?;
    info!(
        "created frontpage document size: {}KB",
        serialized.len() / 1024
    );

    Ok(())
}

async fn build_timeline(
    firestore: &FirestoreApi,
    dated: &[GameEntry],
    undated: &[GameEntry],
) -> Result<(), Status> {
    let today = Utc::now();
    let release_group = |entry: &GameEntry| -> (String, String) {
        let release_date = DateTime::from_timestamp(entry.release_date, 0).unwrap();
        let diff = today.signed_duration_since(release_date);
        let is_future = diff.num_days() < 0;

        let label = if is_future && release_date.month() == 12 && release_date.day() == 31 {
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

    let mut undated = undated
        .into_iter()
        .map(|entry| GameDigest::from(entry.clone()))
        .collect_vec();
    undated.sort_by(|a, b| b.scores.cmp(&a.scores));

    let mut releases = vec![ReleaseEvent {
        label: "?".to_owned(),
        year: "2050".to_owned(),
        games: undated,
    }];

    releases.extend(
        dated
            .into_iter()
            .chunk_by(|entry| release_group(&entry))
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
    info!(
        "created timeline document size: {}KB",
        serialized.len() / 1024
    );

    Ok(())
}

async fn update(resolver_backend: String, entries: Vec<&mut GameEntry>) -> Result<(), Status> {
    let resolver = ResolveApi::new(resolver_backend);
    let firestore = Arc::new(FirestoreApi::connect().await?);
    for game in entries {
        info!("Updating '{}'...", game.name);

        match resolver.retrieve(game.id).await {
            Ok(mut game_entry) => {
                library::firestore::games::write(&firestore, &mut game_entry).await?;
                *game = game_entry
            }
            Err(e) => error!("{e}"),
        }
    }

    Ok(())
}

async fn get_undated(
    firestore: &FirestoreApi,
    notable: &HashSet<String>,
) -> Result<Vec<GameEntry>, Status> {
    let unknown: BoxStream<FirestoreResult<GameEntry>> = firestore
        .db()
        .fluent()
        .select()
        .from("games")
        .filter(|q| q.for_all([q.field(path!(GameEntry::release_date)).equal(0)]))
        .obj()
        .stream_query_with_errors()
        .await?;
    let unknown = unknown.try_collect::<Vec<GameEntry>>().await?;
    info!("unknown = {}", unknown.len());

    let unknown = unknown
        .into_iter()
        .filter(|entry| match entry.category {
            GameCategory::Main
            | GameCategory::Expansion
            | GameCategory::StandaloneExpansion
            | GameCategory::Remake
            | GameCategory::Remaster => true,
            _ => false,
        })
        .filter(|entry| !matches!(entry.status, GameStatus::Cancelled | GameStatus::Rumored))
        .filter(|entry| {
            entry.scores.hype.unwrap_or_default() >= UNKNOWN_HYPE_THRESHOLD
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
    info!("unknown after filtering = {}", unknown.len());

    Ok(unknown)
}

async fn get_upcoming(
    firestore: &FirestoreApi,
    notable: &HashSet<String>,
) -> Result<Vec<GameEntry>, Status> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

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

    Ok(upcoming)
}

async fn get_recent(
    firestore: &FirestoreApi,
    notable: &HashSet<String>,
) -> Result<Vec<GameEntry>, Status> {
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
    let recent = recent.try_collect::<Vec<GameEntry>>().await?;
    info!("recent = {}", recent.len());

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

    Ok(recent)
}

const MONTH_IN_SECONDS: u64 = 30 * 24 * 60 * 60;

const UPCOMING_HYPE_THRESHOLD: u64 = 1;
const UNKNOWN_HYPE_THRESHOLD: u64 = 8;
const EARLY_ACCESS_POPULARITY_THRESHOLD: u64 = 5000;
