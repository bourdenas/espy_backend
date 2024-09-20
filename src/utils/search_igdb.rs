use clap::Parser;
use espy_backend::{resolver::ResolveApi, Tracing};
use itertools::Itertools;

/// IGDB search utility.
#[derive(Parser)]
struct Opts {
    /// Game title to search for in IGDB.
    #[clap(short, long)]
    search: Option<String>,

    /// Game title to search for in IGDB.
    #[clap(long, default_value = "0")]
    id: u64,

    /// URL of the resolver backend.
    #[clap(
        long,
        default_value = "https://resolver-478783154654.europe-west1.run.app"
    )]
    resolver_backend: String,
}

/// Quickly retrieve game info from IGDB based on title or external id matching.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/search_igdb")?;

    let opts: Opts = Opts::parse();
    let resolver = ResolveApi::new(opts.resolver_backend);

    let game_entry = match opts.search {
        Some(title) => {
            let candidates = resolver.search(title, false).await?;

            println!(
                "Found {} candidates.\n{}",
                candidates.len(),
                candidates.iter().map(|game| &game.name).join("\n")
            );
            resolver.retrieve(candidates.first().unwrap().id).await?
        }
        None => resolver.retrieve(opts.id).await?,
    };

    let serialized = serde_json::to_string(&game_entry)?;
    println!("{serialized}");

    Ok(())
}
