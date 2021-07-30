use crate::api::{FirestoreApi, IgdbApi};
use crate::http;
use clap::Clap;
use espy_server::*;
use std::sync::Arc;
use warp::{self, Filter};

#[derive(Clap)]
struct Opts {
    /// JSON file containing application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    /// JSON file containing Firestore credentials for espy service.
    #[clap(
        long,
        default_value = "espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json"
    )]
    firestore_credentials: String,

    /// Port number to use for listening to gRPC requests.
    #[clap(short, long, default_value = "3030")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    let opts: Opts = Opts::parse();

    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();

    let mut igdb = IgdbApi::new(&keys.igdb.client_id, &keys.igdb.secret);
    igdb.connect().await?;

    let _firestore = FirestoreApi::from_credentials(&opts.firestore_credentials);

    println!("starting the HTTP server...");
    warp::serve(
        http::routes::routes(Arc::new(keys), Arc::new(igdb)).with(
            warp::cors()
                .allow_methods(vec!["GET", "POST"])
                .allow_headers(vec!["Content-Type", "Authorization"])
                .allow_any_origin()
                .allow_credentials(true),
        ),
    )
    .run(([127, 0, 0, 1], opts.port))
    .await;

    Ok(())
}
