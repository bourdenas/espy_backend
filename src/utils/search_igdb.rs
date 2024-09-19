use std::sync::Arc;

use clap::Parser;
use espy_backend::{api::IgdbSearch, documents::StoreEntry, *};
use itertools::Itertools;

/// IGDB search utility.
#[derive(Parser)]
struct Opts {
    /// Game title to search for in IGDB.
    #[clap(short, long, default_value = "")]
    search: String,

    /// Game title to search for in IGDB.
    #[clap(long, default_value = "0")]
    id: u64,

    /// External store ID used for retrieving game info.
    #[clap(long, default_value = "")]
    external: String,

    /// If external is set thhis indicates the store name to be used.
    #[clap(long, default_value = "")]
    external_store: String,

    /// If set retrieves all available information for the top candidate of the
    /// search.
    #[clap(long)]
    resolve: bool,

    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,
}

/// Quickly retrieve game info from IGDB based on title or external id matching.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/search_igdb")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = api::IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;
    let igdb = Arc::new(igdb);

    if !&opts.external.is_empty() {
        let game = igdb
            .get_by_store_entry(&StoreEntry {
                id: opts.external,
                storefront_name: opts.external_store,
                ..Default::default()
            })
            .await?;
        println!("Got: {:?}", game);
        return Ok(());
    }

    let games = match opts.id {
        0 => {
            let igdb = Arc::clone(&igdb);
            let igdb_search = IgdbSearch::new(igdb);
            igdb_search.match_by_title(&opts.search).await?
        }
        id => vec![igdb.get(id).await?],
    };

    println!(
        "Found {} candidates.\n{}",
        games.len(),
        games.iter().map(|game| &game.name).join("\n")
    );

    if opts.resolve && !games.is_empty() {
        let firestore = Arc::new(api::FirestoreApi::connect().await?);
        let igdb_game = games.first().unwrap();
        let game_entry = igdb.resolve(firestore, igdb_game.clone()).await?;
        let serialized = serde_json::to_string(&game_entry)?;
        println!("{serialized}");
    }

    Ok(())
}
