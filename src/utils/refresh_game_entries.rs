use std::sync::Arc;

use clap::Parser;
use espy_backend::{api::FirestoreApi, *};
use tracing::{error, info, instrument};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// Refresh only game with specified id.
    #[clap(long)]
    id: Option<u64>,

    /// If set, delete game entry instead of refreshing it.
    #[clap(long)]
    delete: bool,

    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    #[clap(long, default_value = "0")]
    from: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_game_entries")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    let firestore = api::FirestoreApi::connect().await?;

    if let Some(id) = opts.id {
        match opts.delete {
            false => refresh_game(firestore, id, igdb).await?,
            true => library::firestore::games::delete(&firestore, id).await?,
        }
    } else {
        refresh_entries(firestore, igdb, opts.from).await?;
    }

    Ok(())
}

async fn refresh_game(firestore: FirestoreApi, id: u64, igdb: api::IgdbApi) -> Result<(), Status> {
    let game = library::firestore::games::read(&firestore, id).await?;
    refresh(firestore, vec![game], igdb).await
}

#[instrument(level = "trace", skip(firestore, igdb))]
async fn refresh_entries(
    firestore: FirestoreApi,
    igdb: api::IgdbApi,
    from: u64,
) -> Result<(), Status> {
    let game_entries = library::firestore::games::list(&firestore)
        .await?
        .into_iter()
        .skip_while(|e| from != 0 && e.id != from)
        .collect();
    refresh(firestore, game_entries, igdb).await
}

async fn refresh(
    firestore: FirestoreApi,
    game_entries: Vec<documents::GameEntry>,
    igdb: api::IgdbApi,
) -> Result<(), Status> {
    info!("Updating {} game entries...", game_entries.len());
    let mut k = 0;

    let firestore = Arc::new(firestore);
    for game_entry in game_entries {
        info!("#{k} Updating {} ({})", &game_entry.name, game_entry.id);

        match igdb.get(game_entry.id).await {
            Ok(igdb_game) => {
                if let Err(e) = igdb.resolve(Arc::clone(&firestore), igdb_game).await {
                    error!("{e}");
                }
            }
            Err(e) => error!("{e}"),
        }

        k += 1;
    }

    Ok(())
}
