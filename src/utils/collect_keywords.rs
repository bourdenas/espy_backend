use clap::Parser;
use espy_backend::{api, library::firestore, util, Tracing};
use tracing::{error, info};

/// Espy util for refreshing IGDB data for Keywords.
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

    #[clap(long, default_value = "0")]
    offset: u64,

    #[clap(long)]
    count: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/collect_keywords")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;
    let igdb_batch = api::IgdbBatchApi::new(igdb.clone());

    let firestore = api::FirestoreApi::connect().await?;

    let mut k = opts.offset;
    for i in 0.. {
        let keywords = igdb_batch.collect_keywords(opts.offset + i * 500).await?;

        if keywords.len() == 0 {
            break;
        }

        info!(
            "\nWorking on {}:{}",
            opts.offset + i * 500,
            opts.offset + i * 500 + keywords.len() as u64
        );

        if opts.count {
            continue;
        }

        for keyword in keywords {
            if let Err(e) = firestore::keywords::write(&firestore, &keyword).await {
                error!("Failed to save '{}' in Firestore: {e}", &keyword.name);
            }
            info!("#{k} Saved keyword '{}' ({})", keyword.name, keyword.id);

            k += 1;
        }
    }

    Ok(())
}
