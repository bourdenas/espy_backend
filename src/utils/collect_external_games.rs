use clap::Parser;
use espy_backend::{api, documents::ExternalGame, library::firestore, util, Tracing};
use tracing::{error, info};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    #[clap(long, default_value = "0")]
    offset: u64,

    #[clap(long)]
    store: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/collect_external_games")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;
    let igdb_batch = api::IgdbBatchApi::new(igdb.clone());

    let firestore = api::FirestoreApi::connect().await?;

    let mut k = opts.offset;
    for i in 0.. {
        let external_games = igdb_batch
            .collect_external_games(&opts.store, opts.offset + i * 500)
            .await?;
        if external_games.len() == 0 {
            break;
        }
        info!(
            "\nWorking on {}:{}",
            opts.offset + i * 500,
            opts.offset + i * 500 + external_games.len() as u64
        );

        for external_game in external_games {
            let external_game = ExternalGame::from(external_game);
            if let Err(e) = firestore::external_games::write(&firestore, &external_game).await {
                error!(
                    "Failed to save '{}_{}' in Firestore: {e}",
                    &opts.store, external_game.store_id
                );
            }
            k += 1;
        }
    }
    info!("Collected {k} external game mappings for {}", &opts.store);

    Ok(())
}
