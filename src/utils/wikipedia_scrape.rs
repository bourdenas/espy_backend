use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDateTime;
use clap::Parser;
use espy_backend::{
    api::{self, WikipediaScrape},
    documents::{GameEntry, ScoresDoc, WebsiteAuthority},
    library, Status, Tracing,
};
use firestore::{struct_path::path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, StreamExt};
use tracing::error;

#[derive(Parser)]
struct Opts {
    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    #[clap(long, default_value = "0")]
    cursor: u64,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    Tracing::setup("utils/wikipedia_scrape")?;

    let opts: Opts = Opts::parse();

    let mut cursor = opts.cursor;
    let mut i = 0;
    let today = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    while i % BATCH_SIZE == 0 {
        let firestore = Arc::new(api::FirestoreApi::connect().await?);

        let mut game_entries: BoxStream<FirestoreResult<GameEntry>> = firestore
            .db()
            .fluent()
            .select()
            .from("games")
            // .start_at(FirestoreQueryCursor::AfterValue(vec![(&cursor).into()]))
            .filter(|q| {
                q.for_all([
                    q.field(path!(GameEntry::release_date)).less_than(today),
                    q.field(path!(GameEntry::release_date)).greater_than(cursor),
                ])
            })
            .order_by([(
                path!(GameEntry::release_date),
                FirestoreQueryDirection::Ascending,
            )])
            .limit(BATCH_SIZE)
            .obj()
            .stream_query_with_errors()
            .await?;

        while let Some(game_entry) = game_entries.next().await {
            match game_entry {
                Ok(mut game_entry) => {
                    cursor = game_entry.release_date as u64;

                    println!(
                        "#{i} -- {} -- id={} -- release={} ({})",
                        game_entry.name,
                        game_entry.id,
                        game_entry.release_date,
                        NaiveDateTime::from_timestamp_millis(game_entry.release_date * 1000)
                            .unwrap()
                    );

                    let start = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();

                    if game_entry.scores.metacritic.is_none() {
                        let website = game_entry
                            .websites
                            .iter()
                            .find(|e| matches!(e.authority, WebsiteAuthority::Wikipedia));
                        if let Some(website) = website {
                            let response = WikipediaScrape::scrape(&website.url).await;
                            if let Some(response) = response {
                                game_entry.scores.add_wikipedia(response);
                                library::firestore::games::write(&firestore, &mut game_entry)
                                    .await?;

                                let scores = ScoresDoc {
                                    id: game_entry.id,
                                    name: game_entry.name,
                                    scores: game_entry.scores,
                                };
                                library::firestore::scores::write(&firestore, &scores).await?;
                            }
                        }
                    }

                    let finish = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    println!("  -- {} msec", finish - start);
                }
                Err(status) => error!("{status}"),
            }
            i += 1;
        }
    }

    Ok(())
}

const BATCH_SIZE: u32 = 400;
