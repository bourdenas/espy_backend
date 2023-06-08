use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use espy_backend::{
    api,
    documents::{Company, GameDigest},
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

    /// Collect company with specified slug (id).
    #[clap(long)]
    slug: Option<String>,

    #[clap(long, default_value = "0")]
    offset: u64,

    #[clap(long)]
    count: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/collect_companies")?;

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
        let companies = match &opts.slug {
            Some(slug) => igdb_batch.search_company(slug).await?,
            None => {
                igdb_batch
                    .collect_companies(updated_timestamp, opts.offset + i * 500)
                    .await?
            }
        };

        if companies.len() == 0 {
            break;
        }

        info!(
            "\nWorking on {}:{}",
            opts.offset + i * 500,
            opts.offset + i * 500 + companies.len() as u64
        );

        if opts.count {
            continue;
        }

        for igdb_company in companies {
            firestore.validate();
            let mut company = Company {
                id: igdb_company.id,
                name: igdb_company.name,
                slug: igdb_company.slug,
                developed: vec![],
                published: vec![],
            };

            let mut games: HashMap<u64, GameDigest> = HashMap::new();

            for (game_ids, game_digests) in vec![
                (&igdb_company.developed, &mut company.developed),
                (&igdb_company.published, &mut company.published),
            ] {
                for game in game_ids {
                    if let Some(digest) = games.get(game) {
                        game_digests.push(digest.clone());
                        continue;
                    }

                    match firestore::games::read(&firestore, *game) {
                        Ok(game_entry) => {
                            let digest = GameDigest::short_digest(&game_entry);
                            games.insert(digest.id, digest.clone());
                            game_digests.push(digest)
                        }
                        Err(Status::NotFound(_)) => {
                            let digest = match igdb.get_short_digest(*game).await {
                                Ok(digest) => digest,
                                Err(e) => {
                                    error!("  company={}: {e}", &company.name);
                                    continue;
                                }
                            };

                            info!("  #{} fetched '{}' ({})", k, digest.name, digest.id);
                            games.insert(digest.id, digest.clone());
                            game_digests.push(digest)
                        }
                        Err(e) => error!("Failed to read from Firestore game with id={game}: {e}"),
                    }
                }
            }

            if !company.developed.is_empty() || !company.published.is_empty() {
                firestore.validate();
                if let Err(e) = firestore::companies::write(&firestore, &company) {
                    error!("Failed to save '{}' in Firestore: {e}", &company.name);
                }
                info!("#{} Saved company '{}' ({})", k, company.name, company.id);
            }

            k += 1;
        }
    }

    Ok(())
}
