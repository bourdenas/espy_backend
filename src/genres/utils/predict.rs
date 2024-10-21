use std::sync::Arc;

use clap::Parser;
use espy_backend::{api::FirestoreApi, genres::GenrePredictor, library, Status, Tracing};

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

    let game_entry = library::firestore::games::read(&firestore, opts.id).await?;
    let wiki_data = match library::firestore::wikipedia::read(&firestore, opts.id).await {
        Ok(wiki_data) => Some(wiki_data),
        Err(Status::NotFound(_)) => None,
        Err(status) => panic!("{status}"),
    };

    let parent = match &game_entry.parent {
        Some(parent) => match library::firestore::games::read(&firestore, parent.id).await {
            Ok(parent) => Some(parent),
            Err(Status::NotFound(_)) => None,
            Err(status) => panic!("{status}"),
        },
        None => None,
    };
    let parent_wiki_data = match &game_entry.parent {
        Some(parent) => match library::firestore::wikipedia::read(&firestore, parent.id).await {
            Ok(wiki_data) => Some(wiki_data),
            Err(Status::NotFound(_)) => None,
            Err(status) => panic!("{status}"),
        },
        None => None,
    };

    let predictor = GenrePredictor::new(opts.predictor_url);
    if opts.debug {
        let debug_info = predictor
            .debug(&game_entry, wiki_data, parent.as_ref(), parent_wiki_data)
            .await?;
        println!(
            "'{}' ({}) -- debug_info:\n{:?}",
            &game_entry.name, game_entry.id, &debug_info
        );
    } else {
        let genres = predictor
            .predict(&game_entry, wiki_data, parent.as_ref(), parent_wiki_data)
            .await?;
        println!(
            "'{}' ({}) -- espy genres: {:?}",
            &game_entry.name, game_entry.id, &genres
        );
    }

    Ok(())
}
