use std::{
    cmp::min,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use clap::Parser;
use espy_backend::{
    api,
    documents::*,
    library::{
        self,
        firestore::{notable, year},
    },
    webhooks::filtering::GameFilter,
    Tracing,
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
        let filter = GameFilter::new(notable);

        #[derive(Hash, PartialEq, Eq, Debug)]
        enum Partition {
            Releases,
            BelowFold,
            Debug,
        }

        let mut partitions = games.into_iter().into_group_map_by(|game| {
            if filter.apply(&game) {
                if !is_below_fold(game, &filter) {
                    Partition::Releases
                } else if let Some(parent) = &game.parent {
                    if !is_parent_below_fold(&parent, &filter) {
                        Partition::Releases
                    } else {
                        Partition::BelowFold
                    }
                } else {
                    Partition::BelowFold
                }
            } else {
                Partition::Debug
            }
        });

        for (_, digests) in &mut partitions {
            digests.sort_by(|a, b| b.scores.cmp(&a.scores))
        }

        let review = AnnualReview {
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            releases: partitions
                .remove(&Partition::Releases)
                .unwrap_or_default()
                .into_iter()
                .map(|game| GameDigest::from(game))
                .collect(),
            below_fold: partitions
                .remove(&Partition::BelowFold)
                .unwrap_or_default()
                .into_iter()
                .map(|game| GameDigest::from(game))
                .collect(),
            // debug: partitions
            //     .remove(&false)
            //     .unwrap_or_default()
            //     .into_iter()
            //     .map(|game| GameDigest::from(game))
            //     .collect(),
            ..Default::default()
        };

        if opts.cleanup {
            println!("Cleaning up the obsolete entries...");
            let mut i = 0;
            for game in partitions
                .remove(&Partition::Debug)
                .unwrap_or_default()
                .iter()
            {
                println!(
                    "#{i} deleting {}({}) -- {:?}",
                    game.name,
                    game.id,
                    filter.explain(&game)
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

fn is_below_fold(game: &GameEntry, filter: &GameFilter) -> bool {
    matches!(game.status, GameStatus::Cancelled)
        || (game.release_year() >= 2000
            && game.scores.espy_score.is_none()
            && game.scores.popularity.unwrap_or_default() < 1000
            && filter.is_notable(&game).is_none())
}

fn is_parent_below_fold(game: &GameDigest, filter: &GameFilter) -> bool {
    game.release_year() >= 2000
        && game.scores.espy_score.is_none()
        && game.scores.popularity.unwrap_or_default() < 1000
        && !filter
            .is_notable(&GameEntry {
                developers: game
                    .developers
                    .iter()
                    .map(|c| CompanyDigest {
                        name: c.clone(),
                        ..Default::default()
                    })
                    .collect(),
                publishers: game
                    .publishers
                    .iter()
                    .map(|c| CompanyDigest {
                        name: c.clone(),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            })
            .is_none()
}
