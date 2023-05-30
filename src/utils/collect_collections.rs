use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use espy_backend::{
    api,
    documents::{Collection, GameDigest, GameEntry},
    library::firestore,
    util, Tracing,
};
use tracing::{error, info};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    /// JSON file containing Firestore credentials for espy service.
    #[clap(
        long,
        default_value = "espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json"
    )]
    firestore_credentials: String,

    /// Collect only game entries that were updated in the last N days.
    #[clap(long, default_value = "60")]
    updated_since: u64,

    /// Collect collection with specified slug (id).
    #[clap(long)]
    slug: Option<String>,

    #[clap(long, default_value = "0")]
    offset: u64,

    /// If set, look up franchises instead of collections.
    #[clap(long)]
    franchises: bool,

    /// If set, will only count # of collections in IGDB but not try to collect
    /// them.
    #[clap(long)]
    count: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/collect_collections")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;
    let igdb_batch = api::IgdbBatchApi::new(igdb.clone());

    let mut firestore = api::FirestoreApi::from_credentials(opts.firestore_credentials)
        .expect("FirestoreApi.from_credentials()");

    let updated_timestamp = SystemTime::now()
        .checked_sub(Duration::from_secs(24 * 60 * 60 * opts.updated_since))
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut k = opts.offset;
    for i in 0.. {
        let collections = match &opts.slug {
            Some(slug) => match opts.franchises {
                false => igdb_batch.search_collection(slug).await?,
                true => igdb_batch.search_franchises(slug).await?,
            },
            None => match opts.franchises {
                false => {
                    igdb_batch
                        .collect_collections(updated_timestamp, opts.offset + i * 500)
                        .await?
                }
                true => {
                    igdb_batch
                        .collect_franchises(updated_timestamp, opts.offset + i * 500)
                        .await?
                }
            },
        };
        if collections.len() == 0 {
            break;
        }
        info!(
            "\nWorking on {}:{}",
            opts.offset + i * 500,
            opts.offset + i * 500 + collections.len() as u64
        );

        if opts.count {
            continue;
        }

        for collection in collections {
            let mut igdb_collection = Collection {
                id: collection.id,
                name: collection.name,
                slug: collection.slug,
                url: collection.url,
                games: vec![],
            };

            for j in 0.. {
                let games = match opts.franchises {
                    false => {
                        info!("Getting games for collection '{}'", &igdb_collection.slug);
                        igdb_batch
                            .collect_igdb_games_by_collection(igdb_collection.id, j * 500)
                            .await?
                    }
                    true => {
                        info!("Getting games for franchise '{}'", &igdb_collection.slug);
                        igdb_batch
                            .collect_igdb_games_by_franchise(igdb_collection.id, j * 500)
                            .await?
                    }
                };

                info!("  Fetching {} games...", games.len());
                for igdb_game in &games {
                    let cover = match igdb_game.cover {
                        Some(cover_id) => match igdb.get_cover(cover_id).await {
                            Ok(cover) => cover,
                            Err(_) => None,
                        },
                        None => None,
                    };
                    let mut game_entry = GameEntry::from(igdb_game);
                    game_entry.cover = cover;
                    let digest = GameDigest::from(game_entry);

                    info!("  #{} added '{}' ({})", k, digest.name, digest.id);
                    igdb_collection.games.push(digest);
                }

                if games.len() < 500 {
                    break;
                }
            }

            if !igdb_collection.games.is_empty() {
                firestore.validate();
                if let Err(e) = match opts.franchises {
                    false => firestore::collections::write(&firestore, &igdb_collection),
                    true => firestore::franchises::write(&firestore, &igdb_collection),
                } {
                    error!(
                        "Failed to save '{}' in Firestore: {e}",
                        &igdb_collection.name
                    );
                }
                info!(
                    "#{} Saved collection '{}' ({})",
                    k, igdb_collection.name, igdb_collection.id
                );
            }
            k += 1;
        }
    }

    Ok(())
}
