use clap::Parser;
use espy_backend::{
    resolver::{IgdbBatchApi, IgdbConnection},
    util, Tracing,
};

/// Espy util for refreshing IGDB data for Genres.
#[derive(Parser)]
struct Opts {
    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    /// Collect only game entries that were updated in the last N days.
    #[clap(long, default_value = "60")]
    updated_since: u64,

    #[clap(long, default_value = "0")]
    offset: u64,

    #[clap(long)]
    count: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/collect_genres")?;

    let opts: Opts = Opts::parse();
    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let connection = IgdbConnection::new(&keys.igdb.client_id, &keys.igdb.secret).await?;
    let igdb_batch = IgdbBatchApi::new(connection);

    let genres = igdb_batch.collect_genres().await?;
    for genre in genres {
        println!("{:?}", genre);
    }

    Ok(())
}
