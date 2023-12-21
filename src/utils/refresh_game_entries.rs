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
    /// Refresh only game with specified id.
    #[clap(long)]
    id: Option<u64>,

    /// If set, delete game entry instead of refreshing it.
    #[clap(long)]
    delete: bool,

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

    if let Some(id) = opts.id {
        let firestore = Arc::new(api::FirestoreApi::connect().await?);
        if opts.delete {
            library::firestore::games::delete(&firestore, id).await?;
        } else {
            refresh_game(firestore, id, &igdb).await?;
        }
    } else {
        let mut cursor = opts.cursor;
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
                    q.for_all([q.field(path!(GameEntry::id)).greater_than_or_equal(cursor)])
                })
                .order_by([(path!(GameEntry::id), FirestoreQueryDirection::Ascending)])
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
                        if let Err(status) =
                            refresh_game(Arc::clone(&firestore), game_entry.id, &igdb).await
                        {
                            error!("{status}");
                        }
                        cursor = game_entry.id;

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
    id: u64,
    igdb: &api::IgdbApi,
) -> Result<(), Status> {
    let igdb_game = igdb.get(id).await?;
    igdb.resolve(firestore, igdb_game).await?;

    Ok(())
}
