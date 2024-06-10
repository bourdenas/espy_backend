use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use espy_backend::{
    api::FirestoreApi,
    documents::{Collection, GameCategory, GameDigest},
    library, Tracing,
};
use firestore::{struct_path::path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, StreamExt};
use itertools::Itertools;
use tracing::{error, warn};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "0")]
    cursor: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_collections")?;

    let opts: Opts = Opts::parse();
    let mut cursor = opts.cursor;

    let mut i = 0;
    while i % BATCH_SIZE == 0 {
        let firestore = Arc::new(FirestoreApi::connect().await?);

        let mut collections: BoxStream<FirestoreResult<Collection>> = firestore
            .db()
            .fluent()
            .select()
            .from("collections")
            .filter(|q| q.for_all([q.field(path!(Collection::id)).greater_than_or_equal(cursor)]))
            .order_by([(path!(Collection::id), FirestoreQueryDirection::Ascending)])
            .limit(BATCH_SIZE)
            .obj()
            .stream_query_with_errors()
            .await?;

        while let Some(collection) = collections.next().await {
            match collection {
                Ok(collection) => {
                    cursor = collection.id;

                    println!(
                        "#{i} -- {} -- id={} -- {} main games ({} total)",
                        collection.name,
                        collection.id,
                        collection.games.iter().fold(0, |acc, e| acc
                            + match e.category {
                                GameCategory::Main => 1,
                                _ => 0,
                            }),
                        collection.games.len()
                    );

                    let start = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();

                    let (games, missing) = library::firestore::games::batch_read(
                        &firestore,
                        &collection.games.iter().map(|e| e.id).collect_vec(),
                    )
                    .await?;

                    if !missing.is_empty() {
                        warn!(
                            "missing {} GameEntries from collection '{}' ({})",
                            missing.len(),
                            &collection.name,
                            collection.id,
                        );
                    }

                    let collection = Collection {
                        id: collection.id,
                        name: collection.name,
                        slug: collection.slug,
                        url: collection.url,
                        games: games.into_iter().map(|e| GameDigest::from(e)).collect_vec(),
                    };
                    library::firestore::collections::write(&firestore, &collection).await?;

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
