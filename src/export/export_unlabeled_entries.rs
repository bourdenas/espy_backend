use std::{cmp::min, sync::Arc};

use chrono::Utc;
use clap::Parser;
use csv::Writer;
use espy_backend::{
    api::FirestoreApi,
    documents::{GameCategory, GameEntry},
    Tracing,
};
use firestore::{path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, TryStreamExt};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "unlabeled_entries.csv")]
    output: String,

    #[clap(long, default_value = "2023")]
    year: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("export/export_unlabeled_data")?;

    let opts: Opts = Opts::parse();

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

    let firestore = Arc::new(FirestoreApi::connect().await?);

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

    let mut game_entries = game_entries.try_collect::<Vec<GameEntry>>().await?;
    println!("Retrieved {} titles.", game_entries.len());
    game_entries.retain(|game| match game.category {
        GameCategory::Dlc
        | GameCategory::Bundle
        | GameCategory::Episode
        | GameCategory::Version
        | GameCategory::Ignore => false,
        _ => true,
    });
    println!("Retained {} titles.", game_entries.len());

    let examples = game_entries
        .into_iter()
        .map(|entry| UnlabeledExample {
            id: entry.id,
            name: entry.name,
            images: match entry.steam_data {
                Some(steam_data) => steam_data
                    .screenshots
                    .into_iter()
                    .map(|img| img.path_full)
                    .collect_vec(),
                None => entry
                    .screenshots
                    .into_iter()
                    .map(|img| {
                        format!(
                            "https://images.igdb.com/igdb/image/upload/t_720p/{}.png",
                            img.image_id
                        )
                    })
                    .collect_vec(),
            }
            .join("|"),
        })
        .collect_vec();

    let mut writer = Writer::from_path(&opts.output)?;
    for example in examples {
        writer.serialize(example)?;
    }
    writer.flush()?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct UnlabeledExample {
    id: u64,
    name: String,
    images: String,
}
