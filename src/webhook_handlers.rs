use clap::Parser;
use espy_backend::{
    api::{FirestoreApi, IgdbApi},
    library::firestore::notable,
    util,
    webhooks::{self, filtering::GameFilter},
    Status, Tracing,
};
use std::{env, sync::Arc};
use tracing::info;
use warp::{self, Filter};

#[derive(Parser)]
struct Opts {
    /// JSON file containing application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    /// Port number to use for listening to gRPC requests.
    #[clap(short, long, default_value = "8080")]
    port: u16,

    #[clap(long)]
    prod_tracing: bool,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    let opts: Opts = Opts::parse();

    match opts.prod_tracing {
        false => Tracing::setup("espy-webhook-handlers")?,
        true => Tracing::setup_prod("espy-webhook-handlers")?,
    }

    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    let firestore = FirestoreApi::connect().await?;

    // Let ENV VAR override flag.
    let port: u16 = match env::var("PORT") {
        Ok(port) => match port.parse::<u16>() {
            Ok(port) => port,
            Err(_) => opts.port,
        },
        Err(_) => opts.port,
    };

    let notable = notable::read(&firestore).await?;
    let classifier = GameFilter::new(notable);

    info!("webhooks handler started");

    warp::serve(
        webhooks::routes::routes(Arc::new(igdb), Arc::new(firestore), Arc::new(classifier)).with(
            warp::cors()
                .allow_methods(vec!["POST"])
                .allow_headers(vec!["Content-Type", "Authorization"])
                .allow_any_origin()
                .allow_credentials(true),
        ),
    )
    .run(([0, 0, 0, 0], port))
    .await;

    Ok(())
}
