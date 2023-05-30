use clap::Parser;
use espy_backend::{
    api::FirestoreApi,
    documents::{Collection, GameDigest},
    *,
};
use tracing::{error, info, instrument};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// Refresh collection with specified id.
    #[clap(long)]
    id: Option<u64>,

    /// If set, delete game entry instead of refreshing it.
    #[clap(long)]
    delete: bool,

    /// JSON file containing Firestore credentials for espy service.
    #[clap(
        long,
        default_value = "espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json"
    )]
    firestore_credentials: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_collections")?;

    let opts: Opts = Opts::parse();

    let firestore = api::FirestoreApi::from_credentials(opts.firestore_credentials)
        .expect("FirestoreApi.from_credentials()");

    if let Some(id) = opts.id {
        match opts.delete {
            false => refresh_collection(firestore, id).await?,
            true => library::firestore::collections::delete(&firestore, id)?,
        }
    } else {
        refresh_collections(firestore).await?;
    }

    Ok(())
}

async fn refresh_collection(firestore: FirestoreApi, id: u64) -> Result<(), Status> {
    let collection = library::firestore::collections::read(&firestore, id)?;
    refresh(firestore, vec![collection])
}

#[instrument(level = "trace", skip(firestore))]
async fn refresh_collections(firestore: FirestoreApi) -> Result<(), Status> {
    let collections = library::firestore::collections::list(&firestore)?;
    refresh(firestore, collections)
}

fn refresh(mut firestore: FirestoreApi, collections: Vec<Collection>) -> Result<(), Status> {
    info!("Updating {} collections...", collections.len());

    for collection in collections {
        info!("updating {}", &collection.slug);
        firestore.validate();

        let game_digest = collection
            .games
            .into_iter()
            .map(|digest| library::firestore::games::read(&firestore, digest.id))
            .filter_map(|e| e.ok())
            .map(|game_entry| GameDigest::from(game_entry))
            .collect();
        let collection = Collection {
            id: collection.id,
            name: collection.name,
            slug: collection.slug,
            url: collection.url,
            games: game_digest,
        };
        if let Err(e) = library::firestore::collections::write(&firestore, &collection) {
            error!("{e}");
        }
    }

    Ok(())
}
