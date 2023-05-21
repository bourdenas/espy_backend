use std::sync::{Arc, Mutex};

use clap::Parser;
use espy_backend::{
    api::{FirestoreApi, IgdbApi},
    documents::{Library, LibraryEntry},
    games::Reconciler,
    library, util, Status, Tracing,
};
use tracing::{error, info, instrument};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long)]
    user: String,

    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    /// JSON file containing Firestore credentials for espy service.
    #[clap(
        long,
        default_value = "espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json"
    )]
    firestore_credentials: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_library_entries")?;

    let opts: Opts = Opts::parse();
    let firestore = FirestoreApi::from_credentials(opts.firestore_credentials)
        .expect("FirestoreApi.from_credentials()");

    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();
    let mut igdb = IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    refresh_library_entries(firestore, igdb, &opts.user).await?;

    Ok(())
}

#[instrument(level = "trace", skip(firestore, igdb, user_id))]
async fn refresh_library_entries(
    firestore: FirestoreApi,
    igdb: IgdbApi,
    user_id: &str,
) -> Result<(), Status> {
    let legacy_library = library::firestore::library::read(&firestore, user_id)?;
    info!("updating {} titles...", legacy_library.entries.len());

    let firestore = Arc::new(Mutex::new(firestore));
    let mut library = Library { entries: vec![] };
    for entry in legacy_library.entries {
        {
            let mut firestore = firestore.lock().unwrap();
            firestore.validate();
        }
        let game_entry = {
            let firestore = firestore.lock().unwrap();
            library::firestore::games::read(&firestore, entry.id)
        };

        let game_entries = match game_entry {
            Ok(game_entry) => {
                info!("Read from firestore '{title}'", title = game_entry.name);
                Reconciler::expand_bundle(Arc::clone(&firestore), &igdb, game_entry).await
            }
            Err(_) => {
                let game_entry = match igdb.get(entry.id).await {
                    Ok(igdb_game) => {
                        info!("Fetching from igdb '{title}'", title = igdb_game.name);
                        match igdb.get_digest(&igdb_game).await {
                            Ok(game_entry) => game_entry,
                            Err(e) => {
                                error!("Failed to igdb.get_digest: {e}");
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to igdb.get: {e}");
                        continue;
                    }
                };

                Reconciler::expand_bundle(Arc::clone(&firestore), &igdb, game_entry).await
            }
        };

        match game_entries {
            Ok(game_entries) => library.entries.extend(
                game_entries
                    .into_iter()
                    .map(|game_entry| LibraryEntry::new(game_entry, entry.store_entries.clone())),
            ),
            Err(e) => {
                error!("{e}")
            }
        }
    }

    library::firestore::library::write(&firestore.lock().unwrap(), user_id, &library)?;
    let serialized = serde_json::to_string(&library)?;
    info!("updated library size: {}KB", serialized.len() / 1024);

    Ok(())
}
