use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDateTime;
use clap::Parser;
use espy_backend::{api::FirestoreApi, documents::GameEntry, *};
use firestore::{path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, StreamExt};
use tracing::{error, instrument};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    #[clap(long, default_value = "0")]
    cursor: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_game_entries")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

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

                    cursor = game_entry.release_date as u64;

                    let firestore = Arc::clone(&firestore);
                    if let Err(status) = refresh_game(firestore, game_entry, &igdb).await {
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
    skip(firestore, igdb),
    fields(event_span = "resolve_event")
)]
async fn refresh_game(
    firestore: Arc<FirestoreApi>,
    game_entry: GameEntry,
    igdb: &api::IgdbApi,
) -> Result<(), Status> {
    let igdb_game = igdb.get(game_entry.id).await?;
    igdb.resolve(firestore, igdb_game).await?;

    Ok(())
}
