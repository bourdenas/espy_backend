use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use espy_backend::{
    api::{self, FirestoreApi},
    documents::{GameCategory, GameDigest, GameEntry, Timeline},
    library::firestore::timeline,
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

    let firestore = FirestoreApi::connect().await?;

    let notable = timeline::read_notable(&firestore).await?;
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
            entry.scores.popularity.unwrap_or_default() > UPCOMING_POPULARITY_THRESHOLD
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
    let mut recent = recent.try_collect::<Vec<GameEntry>>().await?;
    info!("recent = {}", recent.len());

    let d7 = SystemTime::now()
        .checked_sub(Duration::from_secs(7 * 24 * 60 * 60))
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();
    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    let firestore = Arc::new(firestore);
    for game in &mut recent {
        if game.release_date.unwrap_or_default() as u64 >= d7 {
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
        .filter(|entry| {
            entry
                .developers
                .iter()
                .any(|dev| notable.contains(&dev.name))
                || entry
                    .publishers
                    .iter()
                    .any(|publ| notable.contains(&publ.name))
                || match entry.scores.popularity {
                    Some(value) => match entry.category {
                        GameCategory::Main => value >= RECENT_POPULARITY_THRESHOLD,
                        _ => value >= RECENT_POPULARITY_THRESHOLD_DLC,
                    },
                    None => false,
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

    timeline::write(&firestore, &timeline).await?;

    let serialized = serde_json::to_string(&timeline)?;
    info!("create frontpage size: {}KB", serialized.len() / 1024);

    Ok(())
}

const UPCOMING_POPULARITY_THRESHOLD: u64 = 1;
const RECENT_POPULARITY_THRESHOLD: u64 = 500;
const RECENT_POPULARITY_THRESHOLD_DLC: u64 = 100;
