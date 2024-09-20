use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use espy_backend::{
    api,
    library::firestore,
    resolver::{IgdbBatchApi, IgdbConnection, ResolveApi},
    util, Tracing,
};
use tracing::{error, info};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// Collect only game entries that were updated in the last N days.
    #[clap(long, default_value = "60")]
    updated_since: u64,

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
    Tracing::setup("utils/collect_games")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let connection = IgdbConnection::new(&keys.igdb.client_id, &keys.igdb.secret).await?;
    let igdb_batch = IgdbBatchApi::new(connection);

    let firestore = api::FirestoreApi::connect().await?;
    let resolver = ResolveApi::new(opts.resolver_backend);

    let updated_timestamp = SystemTime::now()
        .checked_sub(Duration::from_secs(24 * 60 * 60 * opts.updated_since))
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut k = opts.offset;
    let mut counter = 0;
    let firestore = Arc::new(firestore);
    for i in 0.. {
        let games = igdb_batch
            .collect_igdb_games(updated_timestamp, opts.offset + i * 500)
            .await?;
        if games.len() == 0 {
            break;
        }
        info!(
            "\nWorking on {}:{}",
            opts.offset + i * 500,
            opts.offset + i * 500 + games.len() as u64
        );

        if opts.count {
            continue;
        }

        for igdb_game in games {
            info!("{k} Processing '{}'", igdb_game.name);
            match firestore::games::read(&firestore, igdb_game.id).await {
                Ok(_) => {}
                Err(_) => match resolver.resolve(igdb_game).await {
                    Ok(game_entry) => {
                        info!("#{} Resolved '{}' ({})", k, game_entry.name, game_entry.id);
                        counter += 1;
                    }
                    Err(e) => {
                        error!("{e}");
                    }
                },
            }

            k += 1;
        }

        info!("Retrieved {counter} new games from IGDB.");
    }

    Ok(())
}
