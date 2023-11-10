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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_collections")?;

    let opts: Opts = Opts::parse();

    let firestore = api::FirestoreApi::connect().await?;

    if let Some(id) = opts.id {
        match opts.delete {
            false => refresh_collection(firestore, id).await?,
            true => library::firestore::collections::delete(&firestore, id).await?,
        }
    } else {
        refresh_collections(firestore).await?;
    }

    Ok(())
}

async fn refresh_collection(firestore: FirestoreApi, id: u64) -> Result<(), Status> {
    let collection = library::firestore::collections::read(&firestore, id).await?;
    refresh(firestore, vec![collection]).await
}

#[instrument(level = "trace", skip(firestore))]
async fn refresh_collections(firestore: FirestoreApi) -> Result<(), Status> {
    let collections = library::firestore::collections::list(&firestore).await?;
    refresh(firestore, collections).await
}

async fn refresh(firestore: FirestoreApi, collections: Vec<Collection>) -> Result<(), Status> {
    info!("Updating {} collections...", collections.len());

    for collection in collections {
        info!("updating {}", &collection.slug);

        let mut game_digest: Vec<GameDigest> = vec![];
        for game in collection.games {
            if let Ok(game_entry) = library::firestore::games::read(&firestore, game.id).await {
                game_digest.push(GameDigest::from(game_entry))
            }
        }

        let collection = Collection {
            id: collection.id,
            name: collection.name,
            slug: collection.slug,
            url: collection.url,
            games: game_digest,
        };
        if let Err(e) = library::firestore::collections::write(&firestore, &collection).await {
            error!("{e}");
        }
    }

    Ok(())
}
