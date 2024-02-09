use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDateTime;
use clap::Parser;
use espy_backend::{api::FirestoreApi, documents::*, library::firestore::year, *};
use firestore::{path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, TryStreamExt};
use itertools::Itertools;
use tracing::instrument;

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
    for game in games {
        println!(
            "#{i} -- {} -- id={} -- release={} ({})",
            game.name,
            game.id,
            game.release_date,
            NaiveDateTime::from_timestamp_millis(game.release_date * 1000).unwrap()
        );

        match game.category {
            GameCategory::Dlc
            | GameCategory::Bundle
            | GameCategory::Episode
            | GameCategory::Version
            | GameCategory::Ignore => {
                continue;
            }
            _ => {}
        }

        i += 1;

        digests.push(GameDigest::from(game))
    }

    let mut partitions = digests.into_iter().into_group_map_by(|digest| {
        if digest.espy_genres.iter().any(|genre| match genre {
            EspyGenre::Indie => digest.scores.thumbs.is_some(),
            _ => false,
        }) {
            "indies"
        } else if let GameStatus::EarlyAccess = digest.status {
            "early_access"
        } else if digest.scores.metacritic.is_some() || digest.scores.thumbs.is_some() {
            "releases"
        } else {
            "debug"
        }
    });

    for (_, digests) in &mut partitions {
        digests.sort_by(|a, b| match b.scores.espy_score.cmp(&a.scores.espy_score) {
            std::cmp::Ordering::Equal => match b.scores.popularity.cmp(&a.scores.popularity) {
                std::cmp::Ordering::Equal => b.scores.thumbs.cmp(&a.scores.thumbs),
                other => other,
            },
            other => other,
        })
    }

    let review = AnnualReview {
        last_updated: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        releases: partitions.remove("releases").unwrap_or(vec![]),
        indies: partitions.remove("indies").unwrap_or(vec![]),
        early_access: partitions.remove("early_access").unwrap_or(vec![]),
        debug: partitions.remove("debug").unwrap_or(vec![]),
    };

    year::write(&firestore, &review, opts.year).await?;

    let serialized = serde_json::to_string(&review)?;
    println!(
        "created annual review for {} size: {}KB",
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
