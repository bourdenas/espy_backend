use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use espy_backend::{
    api::FirestoreApi,
    documents::{Company, GameCategory, GameDigest},
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

    #[clap(long)]
    franchises: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_companies")?;

    let opts: Opts = Opts::parse();
    let mut cursor = opts.cursor;

    let mut i = 0;
    while i % BATCH_SIZE == 0 {
        let firestore = Arc::new(FirestoreApi::connect().await?);

        let mut companies: BoxStream<FirestoreResult<Company>> = firestore
            .db()
            .fluent()
            .select()
            .from("companies")
            .filter(|q| q.for_all([q.field(path!(Company::id)).greater_than_or_equal(cursor)]))
            .order_by([(path!(Company::id), FirestoreQueryDirection::Ascending)])
            .limit(BATCH_SIZE)
            .obj()
            .stream_query_with_errors()
            .await?;

        while let Some(company) = companies.next().await {
            match company {
                Ok(company) => {
                    cursor = company.id;

                    println!(
                        "#{i} -- {} -- id={} -- {} main games ({} total)",
                        company.name,
                        company.id,
                        company.developed.iter().fold(0, |acc, e| acc
                            + match e.category {
                                GameCategory::Main => 1,
                                _ => 0,
                            })
                            + company.published.iter().fold(0, |acc, e| acc
                                + match e.category {
                                    GameCategory::Main => 1,
                                    _ => 0,
                                }),
                        company.developed.len() + company.published.len()
                    );

                    let start = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();

                    let (developed, missing) = library::firestore::games::batch_read(
                        &firestore,
                        &company.developed.iter().map(|e| e.id).collect_vec(),
                    )
                    .await?;

                    if !missing.is_empty() {
                        warn!(
                            "missing {} developed GameEntries from company '{}' ({})",
                            missing.len(),
                            &company.name,
                            company.id,
                        );
                    }

                    let (published, missing) = library::firestore::games::batch_read(
                        &firestore,
                        &company.published.iter().map(|e| e.id).collect_vec(),
                    )
                    .await?;

                    if !missing.is_empty() {
                        warn!(
                            "missing {} published GameEntries from company '{}' ({})",
                            missing.len(),
                            &company.name,
                            company.id,
                        );
                    }

                    let company = Company {
                        id: company.id,
                        name: company.name,
                        slug: company.slug,
                        developed: developed
                            .into_iter()
                            .map(|e| GameDigest::from(e))
                            .collect_vec(),
                        published: published
                            .into_iter()
                            .map(|e| GameDigest::from(e))
                            .collect_vec(),
                    };
                    library::firestore::companies::write(&firestore, &company).await?;

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
