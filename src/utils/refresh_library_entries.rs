use std::{collections::HashMap, sync::Arc};

use clap::Parser;
use csv::Writer;
use espy_backend::{
    api::{FirestoreApi, IgdbApi},
    documents::{GameCategory, GameDigest, Library, LibraryEntry},
    library, util, Status, Tracing,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long)]
    user: String,

    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    #[clap(long)]
    resolve: bool,

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

    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();
    let mut igdb = IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    if !opts.export_csv.is_empty() {
        let library = library::firestore::library::read(&firestore, &opts.user).await?;
        export_library(library, &opts.export_csv)?;
    } else if opts.summary {
        let library = library::firestore::library::read(&firestore, &opts.user).await?;
        let text = summary_library(library);
        println!("{text}");
    } else {
        refresh_library_entries(firestore, igdb, &opts.user, opts.resolve).await?;
    }

    Ok(())
}

#[instrument(level = "trace", skip(firestore, igdb, user_id))]
async fn refresh_library_entries(
    firestore: FirestoreApi,
    igdb: IgdbApi,
    user_id: &str,
    resolve: bool,
) -> Result<(), Status> {
    let legacy_library = library::firestore::library::read(&firestore, user_id).await?;
    info!("updating {} titles...", legacy_library.entries.len());

    let firestore = Arc::new(firestore);

    let mut library_entries: HashMap<u64, LibraryEntry> = HashMap::new();
    let mut k = 0;
    for mut entry in legacy_library.entries {
        println!(
            "#{k} Get '{title}' ({id})",
            title = entry.digest.name,
            id = entry.id,
        );

        let game_entry = if resolve {
            let igdb_game = igdb.get(entry.id).await?;
            igdb.resolve(Arc::clone(&firestore), igdb_game).await
        } else {
            library::firestore::games::read(&firestore, entry.id).await
        };

        let digest = match game_entry {
            Ok(game_entry) => GameDigest::from(game_entry),
            Err(_) => match igdb.get(entry.id).await {
                Ok(igdb_game) => {
                    info!("#{k} Fetching from igdb '{title}'", title = igdb_game.name);
                    match igdb.resolve_digest(&firestore, igdb_game).await {
                        Ok(game_entry) => game_entry,
                        Err(e) => {
                            error!("Failed to igdb.get_digest: {e}");
                            continue;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to igdb.get {}: {e}", entry.id);
                    continue;
                }
            },
        };

        entry.digest = digest;
        library_entries
            .entry(entry.id)
            .and_modify(|e| {
                e.store_entries
                    .extend(entry.store_entries.iter().map(|e| e.clone()))
            })
            .or_insert(entry);
        k += 1;
    }

    let library = Library {
        entries: library_entries
            .into_iter()
            .map(|(_, mut entry)| {
                entry.store_entries.sort_by(|a, b| match a.id.cmp(&b.id) {
                    std::cmp::Ordering::Equal => a.storefront_name.cmp(&b.storefront_name),
                    e => e,
                });
                entry
                    .store_entries
                    .dedup_by(|a, b| a.id == b.id && a.storefront_name == b.storefront_name);
                entry
            })
            .collect(),
    };

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
