use clap::Parser;
use espy_backend::*;
use std::sync::Arc;
use tracing::trace_span;

/// Espy server util for testing functionality of the backend.
#[derive(Parser)]
struct Opts {
    /// Espy user name for managing a game library.
    #[clap(short, long, default_value = "")]
    user: String,

    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,
}

/// Syncs user library with connected storefront retrieving new games and
/// reconciling them.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/sync_library")?;

    let opts: Opts = Opts::parse();

    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;
    let igdb = Arc::new(igdb);

    let firestore = Arc::new(api::FirestoreApi::connect().await?);

    let span = trace_span!("library sync");
    let _guard = span.enter();

    let mut user = library::User::fetch(Arc::clone(&firestore), &opts.user).await?;
    let store_entries = user.sync_accounts(&keys).await?;

    let manager = library::LibraryManager::new(&opts.user);
    manager
        .batch_recon_store_entries(firestore, igdb, store_entries)
        .await?;
    Ok(())
}
