use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use espy_backend::{
    api::{CompanyNormalizer, FirestoreApi},
    documents::{Company, GameDigest},
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

    /// Collect company with specified slug (id).
    #[clap(long)]
    slug: Option<String>,

    #[clap(long, default_value = "0")]
    offset: u64,

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
    Tracing::setup("utils/collect_companies")?;

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
            let mut company = Company {
                id: igdb_company.id,
                slug: CompanyNormalizer::slug(&igdb_company.name),
                name: igdb_company.name,
                logo: String::default(),
                description: igdb_company.description,
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

                    match firestore::games::read(&firestore, *game).await {
                        Ok(game_entry) => {
                            let digest = GameDigest::from(game_entry);
                            games.insert(digest.id, digest.clone());
                            game_digests.push(digest)
                        }
                        Err(Status::NotFound(_)) => {
                            let digest = match resolver.digest(*game).await {
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
                if let Err(e) = firestore::companies::write(&firestore, &company).await {
                    error!("Failed to save '{}' in Firestore: {e}", &company.name);
                }
                info!("#{} Saved company '{}' ({})", k, company.name, company.id);
            }

            k += 1;
        }
    }

    Ok(())
}
