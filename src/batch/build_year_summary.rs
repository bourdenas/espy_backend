use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDateTime;
use clap::Parser;
use espy_backend::{api::FirestoreApi, documents::*, library::firestore::year, *};
use firestore::{path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, TryStreamExt};
use tracing::{info, instrument};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    #[clap(long, default_value = "2023")]
    year: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_game_entries")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    let start = chrono::NaiveDateTime::parse_from_str(
        &format!("{}-01-01 00:00:00", opts.year),
        "%Y-%m-%d %H:%M:%S",
    )?
    .timestamp();
    let end = chrono::NaiveDateTime::parse_from_str(
        &format!("{}-01-01 00:00:00", opts.year + 1),
        "%Y-%m-%d %H:%M:%S",
    )?
    .timestamp();

    let firestore = Arc::new(api::FirestoreApi::connect().await?);

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
    let games = game_entries.try_collect::<Vec<GameEntry>>().await?;

    let mut i = 0;
    let mut digests = vec![];
    for mut game in games {
        println!(
            "#{i} -- {} -- id={} -- release={} ({})",
            game.name,
            game.id,
            game.release_date,
            NaiveDateTime::from_timestamp_millis(game.release_date * 1000).unwrap()
        );
        i += 1;

        if let Some(pop) = game.scores.popularity {
            game.scores.pop_tier = Some(Popularity::create(pop));
        }
        if let Some(thumbs) = game.scores.thumbs {
            game.scores.thumbs_tier = Some(Thumbs::create(thumbs));
        }
        if let Some(critics) = game.scores.metacritic {
            game.scores.critics_tier = Some(Critics::create(critics));
        }
        game.scores.espy_tier = Some(EspyTier::create(&game.scores));

        match game.scores.espy_tier.as_ref().unwrap() {
            EspyTier::Unknown => {}
            _ => digests.push(GameDigest::from(game)),
        }
    }

    let timeline = Timeline {
        last_updated: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        upcoming: vec![],
        recent: digests,
    };

    year::write(&firestore, &timeline, opts.year).await?;

    let serialized = serde_json::to_string(&timeline)?;
    info!(
        "created year {} size: {}KB",
        opts.year,
        serialized.len() / 1024
    );

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
