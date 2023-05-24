use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use espy_backend::{
    api,
    documents::{GameDigest, IgdbCollection},
    library::firestore,
    util, Status, Tracing,
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
            firestore.validate();

            let mut igdb_collection =
                match firestore::collections::read(&firestore, &collection.slug) {
                    Ok(igdb_collection) => igdb_collection,
                    Err(_) => IgdbCollection {
                        id: collection.id,
                        name: collection.name,
                        slug: collection.slug,
                        url: collection.url,
                        games: vec![],
                    },
                };

            for game in &collection.games {
                if let Some(_) = igdb_collection.games.iter().find(|e| e.id == *game) {
                    continue;
                }

                firestore.validate();
                match firestore::games::read(&firestore, *game) {
                    Ok(game_entry) => igdb_collection.games.push(GameDigest::from(game_entry)),
                    Err(Status::NotFound(_)) => {
                        let digest = match igdb.get_short_digest(*game).await {
                            Ok(digest) => digest,
                            Err(e) => {
                                error!("  collection={}: {e}", &igdb_collection.name);
                                continue;
                            }
                        };

                        info!("  #{} fetched '{}' ({})", k, digest.name, digest.id);
                        igdb_collection.games.push(digest)
                    }
                    Err(e) => error!("Failed to read from Firestore game with id={game}: {e}"),
                }
            }

            if !igdb_collection.games.is_empty() {
                firestore.validate();
                if let Err(e) = firestore::collections::write(&firestore, &igdb_collection) {
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
