use std::{collections::HashMap, sync::Arc};

use clap::Parser;
use csv::Writer;
use espy_backend::{
    api::FirestoreApi,
    documents::EspyGenre,
    library::firestore::{games, user_tags},
    Tracing,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long)]
    user: String,

    #[clap(long, default_value = "labeled_entries.csv")]
    output: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("export/export_labeled_data")?;

    let opts: Opts = Opts::parse();

    let firestore = Arc::new(FirestoreApi::connect().await?);
    let tags = user_tags::read(&firestore, &opts.user).await?;

    let mut game_to_genre = HashMap::<u64, Vec<EspyGenre>>::new();
    for genre in tags.genres {
        for id in genre.game_ids {
            game_to_genre
                .entry(id)
                .and_modify(|genres| genres.push(EspyGenre::from_description(genre.name.as_str())))
                .or_insert(vec![EspyGenre::from_description(genre.name.as_str())]);
        }
    }

    let game_ids = game_to_genre.keys().into_iter().map(|e| *e).collect_vec();
    let (game_entries, _) = games::batch_read(&firestore, &game_ids).await?;

    let examples = game_entries
        .into_iter()
        .map(|entry| LabeledExample {
            id: entry.id,
            name: entry.name,
            genres: game_to_genre
                .remove(&entry.id)
                .unwrap_or_default()
                .into_iter()
                .map(|genre| format!("{:?}", genre))
                .join("|"),
            igdb_genres: entry
                .igdb_genres
                .iter()
                .map(|genre| format!("{:?}", genre))
                .join("|"),
            steam_tags: match &entry.steam_data {
                Some(steam_data) => steam_data.user_tags.join("|"),
                None => String::default(),
            },
            images: match entry.steam_data {
                Some(steam_data) => steam_data
                    .screenshots
                    .into_iter()
                    .map(|img| img.path_full)
                    .collect_vec(),
                None => entry
                    .screenshots
                    .into_iter()
                    .map(|img| {
                        format!(
                            "https://images.igdb.com/igdb/image/upload/t_720p/{}.png",
                            img.image_id
                        )
                    })
                    .collect_vec(),
            }
            .join("|"),
        })
        .collect_vec();

    let mut writer = Writer::from_path(&opts.output)?;
    for example in examples {
        writer.serialize(example)?;
    }
    writer.flush()?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct LabeledExample {
    id: u64,
    name: String,
    genres: String,
    igdb_genres: String,
    steam_tags: String,
    images: String,
}
