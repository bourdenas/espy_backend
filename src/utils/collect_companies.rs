use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use espy_backend::{
    api,
    documents::{GameDigest, IgdbCompany},
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
        let companies = igdb_batch
            .collect_companies(updated_timestamp, opts.offset + i * 500)
            .await?;
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

        for company in companies {
            firestore.validate();
            let mut igdb_company = match firestore::companies::read(&firestore, &company.slug) {
                Ok(igdb_company) => igdb_company,
                Err(_) => IgdbCompany {
                    id: company.id,
                    name: company.name,
                    slug: company.slug,
                    developed: vec![],
                    published: vec![],
                },
            };

            for (input, output) in vec![
                (&company.developed, &mut igdb_company.developed),
                (&company.published, &mut igdb_company.published),
            ] {
                for game in input {
                    if let Some(_) = output.iter().find(|e| e.id == *game) {
                        continue;
                    }

                    match firestore::games::read(&firestore, *game) {
                        Ok(game_entry) => output.push(GameDigest::from(game_entry)),
                        Err(Status::NotFound(_)) => {
                            let game_entry = match igdb.get_with_cover(*game).await {
                                Ok(game) => game,
                                Err(e) => {
                                    error!("  company={}: {e}", &igdb_company.name);
                                    continue;
                                }
                            };

                            info!("  #{} fetched '{}' ({})", k, game_entry.name, game_entry.id);
                            output.push(GameDigest::from(game_entry))
                        }
                        Err(e) => error!("Failed to read from Firestore game with id={game}: {e}"),
                    }
                }
            }

            if !igdb_company.developed.is_empty() || !igdb_company.published.is_empty() {
                firestore.validate();
                if let Err(e) = firestore::companies::write(&firestore, &igdb_company) {
                    error!("Failed to save '{}' in Firestore: {e}", &igdb_company.name);
                }
                info!(
                    "#{} Saved company '{}' ({})",
                    k, igdb_company.name, igdb_company.id
                );
            }

            k += 1;
        }
    }

    Ok(())
}
