use std::{collections::HashMap, sync::Arc};

use clap::Parser;
use csv::Writer;
use espy_backend::{
    api::FirestoreApi,
    documents::EspyGenre,
    library::firestore::{games, user_annotations},
    Tracing,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use soup::Soup;

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
    let tags = user_annotations::read(&firestore, &opts.user).await?;

    for genre in &tags.genres {
        println!("{} -- {} examples", &genre.name, genre.game_ids.len());
    }

    let mut game_to_genre = HashMap::<u64, Vec<EspyGenre>>::new();
    for genre in tags.genres {
        for id in genre.game_ids {
            game_to_genre
                .entry(id)
                .and_modify(|genres| genres.push(EspyGenre::from_user_tag(genre.name.as_str())))
                .or_insert(vec![EspyGenre::from_user_tag(genre.name.as_str())]);
        }
    }

    let game_ids = game_to_genre.keys().into_iter().map(|e| *e).collect_vec();
    let games = games::batch_read(&firestore, &game_ids).await?;

    let examples = games
        .documents
        .into_iter()
        .map(|entry| LabeledExample {
            id: entry.id,
            name: entry.name,
            espy_genres: game_to_genre
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
            steam_genres: match &entry.steam_data {
                Some(steam_data) => steam_data.genres.iter().map(|e| &e.description).join("|"),
                None => String::default(),
            },
            gog_genres: match &entry.gog_data {
                Some(gog_data) => gog_data.genres.iter().join("|"),
                None => String::default(),
            },
            igdb_keywords: entry.keywords.join("|"),
            steam_tags: match &entry.steam_data {
                Some(steam_data) => steam_data.user_tags.join("|"),
                None => String::default(),
            },
            gog_tags: match &entry.gog_data {
                Some(gog_data) => gog_data.tags.iter().join("|"),
                None => String::default(),
            },
            description: match &entry.steam_data {
                Some(steam_data) => format!(
                    "{} {}",
                    extract_text(&steam_data.about_the_game),
                    extract_text(&steam_data.detailed_description)
                ),
                None => entry.igdb_game.summary.replace("\n", " "),
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

fn extract_text(html: &str) -> String {
    let soup = Soup::new(&html);
    soup.text().replace("\n", " ")
}

#[derive(Debug, Serialize, Deserialize)]
struct LabeledExample {
    id: u64,
    name: String,
    espy_genres: String,
    igdb_genres: String,
    steam_genres: String,
    gog_genres: String,
    igdb_keywords: String,
    steam_tags: String,
    gog_tags: String,
    description: String,
    images: String,
}
