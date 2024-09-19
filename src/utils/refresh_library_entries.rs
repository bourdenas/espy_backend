use std::collections::HashMap;

use clap::Parser;
use csv::Writer;
use espy_backend::{
    api::FirestoreApi,
    documents::{GameCategory, GameDigest, Library, LibraryEntry},
    library, Status, Tracing,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long)]
    user: String,

    /// Export in a text file the library (for inspection) instead of refreshing it.
    #[clap(long, default_value = "")]
    export_csv: String,

    /// Print a summary of the library (for inspection) instead of refreshing it.
    #[clap(long)]
    summary: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_library_entries")?;

    let opts: Opts = Opts::parse();
    let firestore = FirestoreApi::connect().await?;

    if !opts.export_csv.is_empty() {
        let library = library::firestore::library::read(&firestore, &opts.user).await?;
        export_library(library, &opts.export_csv)?;
    } else if opts.summary {
        let library = library::firestore::library::read(&firestore, &opts.user).await?;
        let text = summary_library(library);
        println!("{text}");
    } else {
        refresh_library_entries(firestore, &opts.user).await?;
    }

    Ok(())
}

#[instrument(level = "trace", skip(firestore, user_id))]
async fn refresh_library_entries(firestore: FirestoreApi, user_id: &str) -> Result<(), Status> {
    let library = library::firestore::library::read(&firestore, user_id).await?;
    info!("updating {} titles...", library.entries.len());

    let result = library::firestore::games::batch_read(
        &firestore,
        &library.entries.iter().map(|e| e.id).collect_vec(),
    )
    .await?;

    if !result.not_found.is_empty() {
        panic!(
            "Entries in library were not found in `/games` collection: {:?}",
            result.not_found
        );
    }

    let library_entries: HashMap<u64, LibraryEntry> =
        HashMap::from_iter(library.entries.into_iter().map(|e| (e.id, e)));

    let entries = result
        .documents
        .into_iter()
        .map(|game_entry| {
            let library_entry = library_entries
                .get(&game_entry.id)
                .expect("Retrieved a game entry that is not present in the user library.");
            LibraryEntry {
                id: game_entry.id,
                digest: GameDigest::from(game_entry),
                store_entries: library_entry.store_entries.clone(),
                added_date: library_entry.added_date,
            }
        })
        .collect_vec();

    let library = Library { entries };
    let serialized = serde_json::to_string(&library)?;
    info!("updated library size: {}KB", serialized.len() / 1024);
    library::firestore::library::write(&firestore, user_id, library).await?;

    Ok(())
}

#[instrument(level = "trace", skip(library))]
fn export_library(
    library: Library,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("exporting {} titles...", library.entries.len());
    let serialized = serde_json::to_string(&library).unwrap();
    info!("library size: {}KB", serialized.len() / 1024);

    let mut writer = Writer::from_path(filename)?;
    for library_entry in library.entries {
        for store_entry in library_entry.store_entries {
            writer.serialize(LibraryRow {
                id: library_entry.id,
                name: library_entry.digest.name.clone(),
                category: library_entry.digest.category,
                storefront: store_entry.storefront_name,
                store_title: store_entry.title,
                store_id: store_entry.id,
                store_url: store_entry.url,
            })?;
        }
    }

    Ok(())
}

#[instrument(level = "trace", skip(library))]
fn summary_library(library: Library) -> String {
    let serialized = serde_json::to_string(&library).unwrap();
    info!("Library contains {} titles", library.entries.len());
    info!("Library size: {}KB", serialized.len() / 1024);
    let categories = group_by_category(library);
    let mut entries: Vec<_> = categories
        .iter()
        .map(|(category, count)| format!("{category:<10}: {count} titles"))
        .collect();
    entries.sort();

    entries.join("\n")
}

fn group_by_category(library: Library) -> HashMap<String, u64> {
    let mut groups = HashMap::<String, u64>::new();

    for entry in library.entries {
        groups
            .entry(entry.digest.category.to_string().to_uppercase())
            .and_modify(|e| *e += 1)
            .or_insert(1);
    }

    groups
}

#[derive(Debug, Serialize, Deserialize)]
struct LibraryRow {
    id: u64,
    name: String,
    category: GameCategory,
    storefront: String,
    store_title: String,
    store_id: String,
    store_url: String,
}
