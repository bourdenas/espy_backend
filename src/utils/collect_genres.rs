use clap::Parser;
use espy_backend::{api, library::firestore, util, Tracing};
use tracing::{error, info};

/// Espy util for refreshing IGDB data for Genres.
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
    Tracing::setup("utils/collect_genres")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;
    let igdb_batch = api::IgdbBatchApi::new(igdb.clone());

    let firestore = api::FirestoreApi::connect().await?;

    let mut k = opts.offset;
    let genres = igdb_batch.collect_genres().await?;

    for genre in genres {
        if let Err(e) = firestore::genres::write(&firestore, &genre).await {
            error!("Failed to save '{}' in Firestore: {e}", &genre.name);
        }
        info!("#{k} Saved genre '{}' ({})", genre.name, genre.id);

        k += 1;
    }

    Ok(())
}
