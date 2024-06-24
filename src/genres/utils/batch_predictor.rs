use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDateTime;
use clap::Parser;
use documents::Genre;
use espy_backend::{documents::GameEntry, *};
use firestore::{path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, StreamExt};
use genres::GenrePredictor;
use tracing::error;

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "0")]
    cursor: u64,

    #[clap(long, default_value = "http://localhost:8080")]
    predictor_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/batch_predictor")?;

    let opts: Opts = Opts::parse();
    let mut cursor = opts.cursor;
    let predictor = GenrePredictor::new(opts.predictor_url);

    let mut i = 0;
    while i % BATCH_SIZE == 0 {
        let firestore = Arc::new(api::FirestoreApi::connect().await?);

        let mut game_entries: BoxStream<FirestoreResult<GameEntry>> = firestore
            .db()
            .fluent()
            .select()
            .from("games")
            .filter(|q| {
                q.for_all([q
                    .field(path!(GameEntry::release_date))
                    .greater_than_or_equal(cursor)])
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

                    let espy_genres = predictor.predict(&game_entry).await?;
                    if !espy_genres.is_empty() {
                        println!("  predicted genres={:?}", &espy_genres);
                        game_entry.espy_genres = espy_genres.clone();

                        library::firestore::genres::write(
                            &firestore,
                            &Genre {
                                game_id: game_entry.id,
                                espy_genres,
                            },
                        )
                        .await?;

                        library::firestore::games::write(&firestore, &mut game_entry).await?;
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

const BATCH_SIZE: u32 = 1000;
