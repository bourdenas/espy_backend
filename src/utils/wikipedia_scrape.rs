use std::sync::Arc;

use clap::Parser;
use espy_backend::{
    api::{self, WikipediaScrape},
    documents::WebsiteAuthority,
    library, Status,
};

#[derive(Parser)]
struct Opts {
    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    /// Game Id to update with wikipedia scrape.
    #[clap(long)]
    id: u64,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    let opts: Opts = Opts::parse();

    let firestore = Arc::new(api::FirestoreApi::connect().await?);

    let mut game = library::firestore::games::read(&firestore, opts.id)
        .await
        .unwrap();

    let website = game
        .websites
        .iter()
        .find(|e| matches!(e.authority, WebsiteAuthority::Wikipedia));
    if let Some(website) = website {
        let response = WikipediaScrape::get_score(&website.url).await;
        if let Some(response) = response {
            game.scores.add_wikipedia(response);
            library::firestore::games::write(&firestore, &mut game).await?;
        }
    }

    Ok(())
}
