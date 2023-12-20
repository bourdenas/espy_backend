use std::sync::Arc;

use clap::Parser;
use espy_backend::{api::FirestoreApi, *};
use firestore::path;
use serde::Deserialize;

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    #[clap(default_value = "games")]
    collections: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/count_docs")?;

    let opts: Opts = Opts::parse();
    let firestore = Arc::new(FirestoreApi::connect().await?);

    for collection in &opts.collections {
        let aggregation: Vec<AggregationStats> = firestore
            .db()
            .fluent()
            .select()
            .from(collection.as_str())
            // .filter(|q| q.for_all([q.field(path!(documents::GameEntry::release_date)).equal(0)]))
            .aggregate(|a| a.fields([a.field(path!(AggregationStats::count)).count()]))
            .obj()
            .query()
            .await?;
        println!(
            "Found {} documents in {collection}",
            aggregation.first().unwrap().count
        );
    }

    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
struct AggregationStats {
    count: usize,
    _sum: Option<usize>,
    _avg: Option<usize>,
}
