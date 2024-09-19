use clap::Parser;
use espy_backend::{
    api::FirestoreApi,
    library::firestore::notable,
    resolver::ResolveApi,
    webhooks::{self, filtering::GameFilter},
    Status, Tracing,
};
use std::{env, sync::Arc};
use tracing::info;
use warp::{self, Filter};

#[derive(Parser)]
struct Opts {
    /// Port number to use for listening to gRPC requests.
    #[clap(short, long, default_value = "8080")]
    port: u16,

    /// URL of the resolver backend.
    #[clap(long, default_value = "")]
    resolver_url: String,

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

    // Let ENV VAR override flag.
    let port: u16 = match env::var("PORT") {
        Ok(port) => match port.parse::<u16>() {
            Ok(port) => port,
            Err(_) => opts.port,
        },
        Err(_) => opts.port,
    };

    let firestore = FirestoreApi::connect().await?;
    let notable = notable::read(&firestore).await?;
    let classifier = GameFilter::new(notable);
    let resolver = ResolveApi::new(opts.resolver_url);

    info!("webhooks handler started");

    warp::serve(
        webhooks::routes::routes(
            Arc::new(firestore),
            Arc::new(resolver),
            Arc::new(classifier),
        )
        .with(
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
