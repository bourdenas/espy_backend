use std::{fs::File, io::BufReader, sync::Arc};

use clap::Parser;
use espy_backend::{api::FirestoreApi, documents::EspyGenre, library::firestore::games, Tracing};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

/// Espy util for updating game genres in batch from a csv file.
#[derive(Parser)]
struct Opts {
    #[clap(long)]
    predictions: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("export/import_labeled_data")?;

    let opts: Opts = Opts::parse();

    let firestore = Arc::new(FirestoreApi::connect().await?);

    let csv_file = BufReader::new(File::open(&opts.predictions)?);
    let mut reader = csv::Reader::from_reader(csv_file);

    // Iterate over each record in the CSV
    let mut examples = vec![];
    for result in reader.deserialize::<ExamplePrediction>() {
        examples.push(result?);
    }

    let game_ids = examples.iter().map(|e| e.id).collect_vec();
    let (mut game_entries, _) = games::batch_read(&firestore, &game_ids).await?;

    for game_entry in &mut game_entries {
        let example = examples.iter().find(|e| e.id == game_entry.id).unwrap();

        let genres = example
            .prediction
            .split(",")
            .map(|e| EspyGenre::from(e))
            .collect_vec();
        game_entry.espy_genres = genres;

        println!("Updating {}", &game_entry.name);
        games::write(&firestore, game_entry).await?;
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct ExamplePrediction {
    id: u64,
    name: String,
    prediction: String,
    genres: String,
    features: String,
}
