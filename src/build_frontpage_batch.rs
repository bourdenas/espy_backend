use clap::Parser;
use espy_backend::{
    documents::{Frontpage, GameCategory, GameDigest, GameEntry},
    Status, Tracing,
};
use firestore::{path, FirestoreDb, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, TryStreamExt};
use itertools::Itertools;
use std::{
    cmp::min,
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
        false => Tracing::setup("build-frontpage-batch")?,
        true => Tracing::setup_prod("build-frontpage-batch")?,
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
        .collect_vec();
    info!("recent after filtering = {}", recent.len());

    let frontpage = Frontpage {
        last_updated: now,
        upcoming: upcoming
            .iter()
            .take(50)
            .map(|game_entry| GameDigest::from(game_entry.clone()))
            .collect(),
        most_anticipated: upcoming
            .into_iter()
            .filter(|entry| entry.popularity.is_some())
            .sorted_by(|a, b| Ord::cmp(&b.popularity.unwrap(), &a.popularity.unwrap()))
            .map(|game_entry| GameDigest::from(game_entry))
            .take(50)
            .collect(),
        recent: recent
            .iter()
            .take(50)
            .map(|game_entry| GameDigest::from(game_entry.clone()))
            .collect(),
        popular: recent
            .iter()
            .filter(|entry| entry.popularity.is_some())
            .sorted_by(|a, b| Ord::cmp(&b.popularity.unwrap(), &a.popularity.unwrap()))
            .map(|game_entry| GameDigest::from(game_entry.clone()))
            .take(50)
            .collect(),
        critically_acclaimed: recent
            .into_iter()
            .filter(|entry| {
                entry.score.is_some()
                    && match entry.popularity {
                        Some(popularity) => popularity > 1000,
                        None => false,
                    }
            })
            .sorted_by(|a, b| {
                (b.score.unwrap() as f64 * (min(b.popularity.unwrap(), 10000) as f64 / 10000.0))
                    .total_cmp(
                        &(a.score.unwrap() as f64
                            * (min(a.popularity.unwrap(), 10000) as f64 / 10000.0)),
                    )
            })
            .map(|game_entry| GameDigest::from(game_entry))
            .take(50)
            .collect(),
    };

    db.fluent()
        .update()
        .in_col("espy")
        .document_id("frontpage")
        .object(&frontpage)
        .execute()
        .await?;

    Ok(())
}
