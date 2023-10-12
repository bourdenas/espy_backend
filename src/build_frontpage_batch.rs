use clap::Parser;
use espy_backend::{
    documents::{Frontpage, GameDigest, GameEntry},
    Status, Tracing,
};
use firestore::{path, FirestoreDb, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, TryStreamExt};
use itertools::Itertools;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
            .filter(|entry| entry.score.is_some())
            .sorted_by(|a, b| Ord::cmp(&b.score.unwrap(), &a.score.unwrap()))
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
