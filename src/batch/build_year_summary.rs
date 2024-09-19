use std::{
    cmp::min,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use clap::Parser;
use espy_backend::{
    documents::*,
    library::firestore::{notable, year},
    webhooks::filtering::{GameEntryClass, GameFilter},
    *,
};
use firestore::{path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, TryStreamExt};
use itertools::Itertools;

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long)]
    year: Option<u64>,

    #[clap(long, default_value = "false")]
    cleanup: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_game_entries")?;

    let opts: Opts = Opts::parse();

    let start_year = match opts.year {
        Some(year) => year,
        None => 1979,
    };
    let end_year = match opts.year {
        Some(year) => year + 1,
        None => 2025,
    };

    for year in start_year..end_year {
        println!("Building library for year {year}...");

        let start = chrono::DateTime::parse_from_str(
            &format!("{}-01-01 00:00:00 +0000", year),
            "%Y-%m-%d %H:%M:%S %z",
        )
        .expect("Failed to parse start date")
        .timestamp();
        let end = min(
            chrono::DateTime::parse_from_str(
                &format!("{}-01-01 00:00:00 +0000", year + 1),
                "%Y-%m-%d %H:%M:%S %z",
            )
            .expect("Failed to parse end date")
            .timestamp(),
            Utc::now().timestamp(),
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

        games.retain(|game| game.category.is_main_category());
        println!("Retained {} titles.", games.len());

        let notable = notable::read(&firestore).await?;
        let classifier = GameFilter::new(notable);

        let mut partitions = games
            .into_iter()
            .into_group_map_by(|game| classifier.classify(&game));

        for (_, digests) in &mut partitions {
            digests.sort_by(|a, b| b.scores.cmp(&a.scores))
        }

        let review = AnnualReview {
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            releases: partitions
                .remove(&GameEntryClass::Main)
                .unwrap_or_default()
                .into_iter()
                .map(|game| GameDigest::from(game))
                .collect(),
            indies: partitions
                .remove(&GameEntryClass::Indie)
                .unwrap_or_default()
                .into_iter()
                .map(|game| GameDigest::from(game))
                .collect(),
            remasters: partitions
                .remove(&GameEntryClass::Remaster)
                .unwrap_or_default()
                .into_iter()
                .map(|game| GameDigest::from(game))
                .collect(),
            expansions: partitions
                .remove(&GameEntryClass::Expansion)
                .unwrap_or_default()
                .into_iter()
                .map(|game| GameDigest::from(game))
                .collect(),
            casual: partitions
                .remove(&GameEntryClass::Casual)
                .unwrap_or_default()
                .into_iter()
                .map(|game| GameDigest::from(game))
                .collect(),
            early_access: partitions
                .remove(&GameEntryClass::EarlyAccess)
                .unwrap_or_default()
                .into_iter()
                .map(|game| GameDigest::from(game))
                .collect(),
            debug: partitions
                .remove(&GameEntryClass::Debug)
                .unwrap_or_default()
                .into_iter()
                .map(|game| GameDigest::from(game))
                .collect(),
        };

        if opts.cleanup {
            println!("Cleaning up the obsolete entries...");
            let mut i = 0;
            for game in partitions
                .remove(&GameEntryClass::Ignore)
                .unwrap_or_default()
                .iter()
            {
                println!(
                    "#{i} deleting {}({}) -- {}",
                    game.name,
                    game.id,
                    classifier.explain(&game)
                );
                i += 1;
                library::firestore::games::delete(&firestore, game.id).await?;
            }
        }

        year::write(&firestore, &review, year).await?;

        let serialized = serde_json::to_string(&review)?;
        println!(
            "Created annual review for {year} size: {}KB\n",
            serialized.len() / 1024
        );
    }

    Ok(())
}
