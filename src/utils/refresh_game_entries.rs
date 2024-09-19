use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::DateTime;
use clap::Parser;
use espy_backend::{documents::GameEntry, *};
use firestore::{path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, StreamExt};
use resolver::ResolveApi;
use tracing::{error, instrument};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// URL of the resolver backend.
    #[clap(long, default_value = "")]
    resolver_backend: String,

    #[clap(long, default_value = "0")]
    cursor: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_game_entries")?;

    let opts: Opts = Opts::parse();
    let resolver = ResolveApi::new(opts.resolver_backend);

    let mut cursor = match opts.cursor {
        0 => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        cursor => cursor,
    };
    let mut i = 0;
    while i % 400 == 0 {
        let firestore = Arc::new(api::FirestoreApi::connect().await?);

        let mut game_entries: BoxStream<FirestoreResult<GameEntry>> = firestore
            .db()
            .fluent()
            .select()
            .from("games")
            // .start_at(FirestoreQueryCursor::AfterValue(vec![(&cursor).into()]))
            .filter(|q| {
                q.for_all([
                    q.field(path!(GameEntry::release_date))
                        .less_than_or_equal(cursor),
                    q.field(path!(GameEntry::release_date)).greater_than(0),
                ])
            })
            .order_by([(
                path!(GameEntry::release_date),
                FirestoreQueryDirection::Descending,
            )])
            .limit(400)
            .obj()
            .stream_query_with_errors()
            .await?;

        while let Some(game_entry) = game_entries.next().await {
            match game_entry {
                Ok(game_entry) => {
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

                    if let Err(status) = refresh_game(game_entry, &resolver).await {
                        error!("{status}");
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

#[instrument(
    level = "info",
    skip(game_entry, resolver),
    fields(event_span = "resolve_event")
)]
async fn refresh_game(game_entry: GameEntry, resolver: &ResolveApi) -> Result<(), Status> {
    resolver.retrieve(game_entry.id).await?;
    Ok(())
}
