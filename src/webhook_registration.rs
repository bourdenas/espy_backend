use clap::Parser;
use espy_backend::{
    resolver::{endpoints, IgdbConnection},
    util, Status, Tracing,
};
use reqwest::StatusCode;
use tracing::info;

#[derive(Parser)]
struct Opts {
    /// JSON file containing application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    /// URL of the webhooks http backend.
    #[clap(long, default_value = "https://webhooks-fjxkoqq4wq-ew.a.run.app")]
    webhooks_backend: String,

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

    info!("webhooks registration");
    let connection = IgdbConnection::new(&keys.igdb.client_id, &keys.igdb.secret).await?;
    let webhooks_api = IgdbWebhooksApi::new(connection);
    webhooks_api
        .register_games_webhook(&opts.webhooks_backend, "foo")
        .await?;

    Ok(())
}

pub struct IgdbWebhooksApi {
    connection: IgdbConnection,
}

impl IgdbWebhooksApi {
    pub fn new(connection: IgdbConnection) -> IgdbWebhooksApi {
        IgdbWebhooksApi { connection }
    }

    pub async fn register_games_webhook(
        &self,
        webhook_url: &str,
        secret: &str,
    ) -> Result<(), Status> {
        create_webhook(
            &self.connection,
            endpoints::GAMES,
            &format!("{webhook_url}/add_game"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::GAMES,
            &format!("{webhook_url}/update_game"),
            "update",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::EXTERNAL_GAMES,
            &format!("{webhook_url}/external_games"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::EXTERNAL_GAMES,
            &format!("{webhook_url}/external_games"),
            "update",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::GENRES,
            &format!("{webhook_url}/genres"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::GENRES,
            &format!("{webhook_url}/genres"),
            "update",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::KEYWORDS,
            &format!("{webhook_url}/keywords"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::KEYWORDS,
            &format!("{webhook_url}/keywords"),
            "update",
            secret,
        )
        .await
    }
}

async fn create_webhook(
    connection: &IgdbConnection,
    endpoint: &str,
    webhook_url: &str,
    method: &str,
    secret: &str,
) -> Result<(), Status> {
    connection.qps.wait();

    let _permit = connection.qps.connection().await;
    let uri = format!("{IGDB_SERVICE_URL}/{endpoint}/webhooks");
    let resp = reqwest::Client::new()
        .post(&uri)
        .header("Client-ID", &connection.client_id)
        .header(
            "Authorization",
            format!("Bearer {}", &connection.oauth_token),
        )
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("url={webhook_url}&secret={secret}&method={method}"))
        .send()
        .await?;

    match resp.status() {
        StatusCode::OK => {
            let text = resp.text().await?;
            info!("Webhook registration response: {text}");
            Ok(())
        }
        _ => {
            let text = resp.text().await?;
            Err(Status::internal(format!(
                "Webhook registration failed: {text}"
            )))
        }
    }
}

const IGDB_SERVICE_URL: &str = "https://api.igdb.com/v4";
