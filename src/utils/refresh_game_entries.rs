use clap::Parser;
use espy_backend::{api::FirestoreApi, library::firestore, *};
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
    offset: u64,

    /// JSON file containing Firestore credentials for espy service.
    #[clap(
        long,
        default_value = "espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json"
    )]
    firestore_credentials: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_game_entries")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    let steam = games::SteamDataApi::new();

    let firestore = api::FirestoreApi::from_credentials(opts.firestore_credentials)
        .expect("FirestoreApi.from_credentials()");

    if let Some(id) = opts.id {
        match opts.delete {
            false => refresh_game(firestore, id, igdb, steam).await?,
            true => library::firestore::games::delete(&firestore, id)?,
        }
    } else {
        refresh_entries(firestore, igdb, steam, opts.offset).await?;
    }

    Ok(())
}

async fn refresh_game(
    firestore: FirestoreApi,
    id: u64,
    igdb: api::IgdbApi,
    steam: games::SteamDataApi,
) -> Result<(), Status> {
    let game = library::firestore::games::read(&firestore, id)?;
    refresh(firestore, vec![game], igdb, steam).await
}

#[instrument(level = "trace", skip(firestore, igdb, steam))]
async fn refresh_entries(
    firestore: FirestoreApi,
    igdb: api::IgdbApi,
    steam: games::SteamDataApi,
    offset: u64,
) -> Result<(), Status> {
    let game_entries = library::firestore::games::list(&firestore)?
        .into_iter()
        .skip_while(|e| e.id != offset)
        .collect();
    refresh(firestore, game_entries, igdb, steam).await
}

async fn refresh(
    mut firestore: FirestoreApi,
    game_entries: Vec<documents::GameEntry>,
    igdb: api::IgdbApi,
    steam: games::SteamDataApi,
) -> Result<(), Status> {
    info!("Updating {} game entries...", game_entries.len());
    let mut k = 0;

    for game_entry in game_entries {
        info!("#{k} Updating {} ({})", &game_entry.name, game_entry.id);
        firestore.validate();

        if game_entry.igdb_hypes == 0 && game_entry.igdb_follows == 0 {
            if let Err(e) = firestore::games::delete(&firestore, game_entry.id) {
                error!("{e}");
            }
        } else {
            let igdb_game = match igdb.get(game_entry.id).await {
                Ok(game) => game,
                Err(e) => {
                    error!("{e}");
                    k += 1;
                    continue;
                }
            };

            let mut game_entry = match igdb.resolve(igdb_game).await {
                Ok(game_entry) => game_entry,
                Err(e) => {
                    error!("{e}");
                    k += 1;
                    continue;
                }
            };

            if let Err(e) = steam.retrieve_steam_data(&mut game_entry).await {
                error!("Failed to retrieve SteamData for '{}' {e}", game_entry.name);
            }

            if let Err(e) = firestore::games::write(&firestore, &game_entry) {
                error!("Failed to save '{}' in Firestore: {e}", game_entry.name);
            }
        }
        k += 1;
    }

    Ok(())
}
