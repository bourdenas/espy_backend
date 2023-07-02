use clap::Parser;
use espy_backend::{
    api::{IgdbApi, IgdbWebhooksApi},
    util, Status, Tracing,
};
use tracing::info;

#[derive(Parser)]
struct Opts {
    /// JSON file containing application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    #[clap(long)]
    prod_tracing: bool,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    let opts: Opts = Opts::parse();

    match opts.prod_tracing {
        false => Tracing::setup("espy-webhook-registration")?,
        true => Tracing::setup_prod("espy-webhook-registration")?,
    }

    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    info!("webhooks registration");
    let webhooks_api = IgdbWebhooksApi::new(igdb.clone());
    webhooks_api
        .register_games_webhook("https://webhooks-fjxkoqq4wq-ew.a.run.app", "foo")
        .await?;

    Ok(())
}
