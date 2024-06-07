use std::sync::Arc;

use clap::Parser;
use espy_backend::{api::FirestoreApi, genres::GenrePredictor, library::firestore::games, Tracing};

/// Espy util for quickly checking the espy genre prediction for a GameEntry.
#[derive(Parser)]
struct Opts {
    #[clap(long)]
    id: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/predict")?;

    let opts: Opts = Opts::parse();

    let firestore = Arc::new(FirestoreApi::connect().await?);

    let game_entry = games::read(&firestore, opts.id).await?;
    let genres = GenrePredictor::predict(&game_entry).await?;
    println!(
        "'{}' ({}) -- espy genres: {:?}",
        &game_entry.name, game_entry.id, &genres
    );

    Ok(())
}
