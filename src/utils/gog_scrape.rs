use clap::Parser;
use espy_backend::{
    api::{self, GogScrape},
    documents::WebsiteAuthority,
    library, Status, Tracing,
};

#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "0")]
    id: u64,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    Tracing::setup("utils/gog_scrape")?;

    let opts: Opts = Opts::parse();

    let firestore = api::FirestoreApi::connect().await?;
    let game_entry = library::firestore::games::read(&firestore, opts.id).await?;

    let website = game_entry
        .websites
        .iter()
        .find(|e| matches!(e.authority, WebsiteAuthority::Gog));
    if let Some(website) = website {
        let gog_data = GogScrape::scrape(&website.url).await;
        println!("gog_date={:#?}", gog_data);
    }

    Ok(())
}
