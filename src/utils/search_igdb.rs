use clap::Parser;
use espy_backend::{documents::StoreEntry, *};
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
    expand: bool,

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
        0 => igdb.search_by_title(&opts.search).await?,
        id => vec![igdb.get(id).await?],
    };

    println!(
        "Found {} candidates.\n{}",
        games.len(),
        games.iter().map(|game| &game.name).join("\n")
    );

    if opts.expand && !games.is_empty() {
        let igdb_game = games.first().unwrap();

        println!("{:?}", igdb_game);
        if igdb_game.category == 3 || igdb_game.version_parent.is_some() {
            let games = igdb.expand_bundle(igdb_game.id).await?;
            println!("\nincludes:");
            for igdb_game in games {
                println!("{:?}", igdb_game);
            }
        }
    }

    Ok(())
}
