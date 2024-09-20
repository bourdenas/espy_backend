use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use espy_backend::{
    api::FirestoreApi,
    documents::{Collection, GameDigest, GameEntry},
    library::firestore,
    resolver::{IgdbBatchApi, IgdbConnection, ResolveApi},
    util, Status, Tracing,
};
use tracing::{error, info};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
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

    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    /// URL of the resolver backend.
    #[clap(
        long,
        default_value = "https://resolver-478783154654.europe-west1.run.app"
    )]
    resolver_backend: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/collect_collections")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let connection = IgdbConnection::new(&keys.igdb.client_id, &keys.igdb.secret).await?;
    let igdb_batch = IgdbBatchApi::new(connection);

    let firestore = FirestoreApi::connect().await?;
    let resolver = ResolveApi::new(opts.resolver_backend);

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
                    match firestore::games::read(&firestore, igdb_game.id).await {
                        Ok(game_entry) => {
                            igdb_collection.games.push(GameDigest::from(game_entry));
                        }
                        Err(Status::NotFound(_)) => {
                            let digest = match resolver.digest(igdb_game.id).await {
                                Ok(digest) => digest,
                                Err(e) => {
                                    error!("  collection={}: {e}", &igdb_collection.name);
                                    continue;
                                }
                            };

                            info!("  #{} fetched '{}' ({})", k, digest.name, digest.id);
                            igdb_collection.games.push(digest);
                        }
                        Err(e) => error!(
                            "Failed to read from Firestore game with id={}: {e}",
                            igdb_game.id
                        ),
                    }

                    let digest = GameDigest::from(GameEntry::from(igdb_game.clone()));
                    info!("  #{} added '{}' ({})", k, digest.name, digest.id);
                    igdb_collection.games.push(digest);
                }

                if games.len() < 500 {
                    break;
                }
            }

            if !igdb_collection.games.is_empty() {
                if let Err(e) = match opts.franchises {
                    false => firestore::collections::write(&firestore, &igdb_collection).await,
                    true => firestore::franchises::write(&firestore, &igdb_collection).await,
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
