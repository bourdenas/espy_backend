use crate::Status;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use tracing::{error, info};

use super::IgdbConnection;

/// Sends a POST request to an IGDB service endpoint.
pub async fn post<T: DeserializeOwned>(
    connection: &IgdbConnection,
    endpoint: &str,
    body: &str,
) -> Result<T, Status> {
    connection.qps.wait();

    let _permit = connection.qps.connection().await;
    let uri = format!("{IGDB_SERVICE_URL}/{endpoint}/");
    let resp = reqwest::Client::new()
        .post(&uri)
        .header("Client-ID", &connection.client_id)
        .header(
            "Authorization",
            format!("Bearer {}", &connection.oauth_token),
        )
        .body(String::from(body))
        .send()
        .await?;

    let text = resp.text().await?;
    let resp = serde_json::from_str::<T>(&text).map_err(|_| {
        let msg = format!("Received unexpected response: {text}\nuri: {uri}\nquery: {body}");
        error!(msg);
        Status::internal(msg)
    });

    resp
}

pub async fn create_webhook(
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
