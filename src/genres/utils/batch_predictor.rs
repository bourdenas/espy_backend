use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::DateTime;
use clap::Parser;
use documents::Genre;
use espy_backend::{documents::GameEntry, *};
use firestore::{path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, StreamExt};
use genres::GenrePredictor;
use library::firestore::wikipedia;
use tracing::{error, warn};

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
                        DateTime::from_timestamp_millis(game_entry.release_date * 1000).unwrap()
                    );

                    let start = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();

                    let wiki_data = match wikipedia::read(&firestore, game_entry.id).await {
                        Ok(wiki_data) => Some(wiki_data),
                        Err(Status::NotFound(_)) => None,
                        Err(status) => panic!("{status}"),
                    };

                    let mut parent = match &game_entry.parent {
                        Some(parent) => {
                            match library::firestore::games::read(&firestore, parent.id).await {
                                Ok(parent) => Some(parent),
                                Err(Status::NotFound(_)) => {
                                    warn!(
                                        "Missing parent '{}' ({}) for entry '{}' ({})",
                                        parent.name, parent.id, game_entry.name, game_entry.id
                                    );
                                    None
                                }
                                Err(status) => {
                                    warn!(
                                        "Failed to retrieve parent for entry '{}' ({}): {status}",
                                        game_entry.name, game_entry.id
                                    );
                                    None
                                }
                            }
                        }
                        None => None,
                    };
                    let parent_wiki_data = match &game_entry.parent {
                        Some(parent) => match wikipedia::read(&firestore, parent.id).await {
                            Ok(wiki_data) => Some(wiki_data),
                            Err(Status::NotFound(_)) => None,
                            Err(status) => panic!("{status}"),
                        },
                        None => None,
                    };

                    let espy_genres = predictor
                        .predict(&game_entry, wiki_data, parent.as_ref(), parent_wiki_data)
                        .await?;

                    if !espy_genres.is_empty() {
                        println!("  predicted genres={:?}", &espy_genres);
                        game_entry.espy_genres = espy_genres.clone();

                        if let Some(parent) = &mut parent {
                            parent.espy_genres = espy_genres.clone();
                        }

                        library::firestore::genres::write(
                            &firestore,
                            &Genre {
                                game_id: game_entry.id,
                                espy_genres,
                            },
                        )
                        .await?;

                        library::firestore::games::write(&firestore, &mut game_entry).await?;
                        if let Some(parent) = &mut parent {
                            library::firestore::games::write(&firestore, parent).await?;
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

const BATCH_SIZE: u32 = 1000;
