use std::sync::Arc;

use clap::Parser;
use espy_backend::{api::FirestoreApi, genres::GenrePredictor, library::firestore::games, Tracing};

/// Espy util for quickly checking the espy genre prediction for a GameEntry.
#[derive(Parser)]
struct Opts {
    #[clap(long)]
    id: u64,

    #[clap(long, default_value = "http://localhost:8080")]
    predictor_url: String,

    #[clap(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/predict")?;

    let opts: Opts = Opts::parse();

    let firestore = Arc::new(FirestoreApi::connect().await?);

    let game_entry = games::read(&firestore, opts.id).await?;

    let predictor = GenrePredictor::new(opts.predictor_url);
    if opts.debug {
        let debug_info = predictor.debug(&game_entry).await?;
        println!(
            "'{}' ({}) -- debug_info:\n{:?}",
            &game_entry.name, game_entry.id, &debug_info
        );
    } else {
        let genres = predictor.predict(&game_entry).await?;
        println!(
            "'{}' ({}) -- espy genres: {:?}",
            &game_entry.name, game_entry.id, &genres
        );
    }

    Ok(())
}
