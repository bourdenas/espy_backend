use clap::Parser;
use espy_backend::{api::FirestoreApi, http, resolver::ResolveApi, util, Status, Tracing};
use std::{env, sync::Arc};
use warp::{self, Filter};

#[derive(Parser)]
struct Opts {
    /// JSON file containing application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    /// Port number to use for listening to gRPC requests.
    #[clap(short, long, default_value = "8080")]
    port: u16,

    /// URL of the resolver backend.
    #[clap(
        long,
        default_value = "https://resolver-478783154654.europe-west1.run.app"
    )]
    resolver_backend: String,

    #[clap(long)]
    prod_tracing: bool,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    let opts: Opts = Opts::parse();

    match opts.prod_tracing {
        false => Tracing::setup("espy-httpserver")?,
        true => Tracing::setup_prod("espy-library", "query_logs")?,
    }

    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    // Let ENV VAR override flag.
    let port: u16 = match env::var("PORT") {
        Ok(port) => match port.parse::<u16>() {
            Ok(port) => port,
            Err(_) => opts.port,
        },
        Err(_) => opts.port,
    };

    let firestore = FirestoreApi::connect().await?;
    let resolver = ResolveApi::new(opts.resolver_backend);

    warp::serve(
        http::routes::routes(Arc::new(keys), Arc::new(firestore), Arc::new(resolver)).with(
            warp::cors()
                .allow_methods(vec!["GET", "POST"])
                .allow_headers(vec!["Content-Type", "Authorization"])
                .allow_any_origin()
                .allow_credentials(true),
        ),
    )
    .run(([0, 0, 0, 0], port))
    .await;

    Ok(())
}
