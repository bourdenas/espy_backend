use serde::de::DeserializeOwned;

use crate::{logging::IgdbRequestCounter, Status};

use super::IgdbConnection;

/// Sends a POST request to an IGDB service endpoint.
pub async fn post<T: DeserializeOwned>(
    connection: &IgdbConnection,
    endpoint: &str,
    body: &str,
) -> Result<T, Status> {
    connection.qps.wait();

    let counter = IgdbRequestCounter::new(endpoint);

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
        .await;

    let resp = match resp {
        Ok(resp) => resp,
        Err(e) => {
            let status =
                Status::internal(format!("Request failed: {e}\nuri: {uri}\nquery: {body}"));
            counter.log_error(&status);
            return Err(status);
        }
    };

    let text = resp.text().await?;
    match serde_json::from_str::<T>(&text) {
        Ok(resp) => {
            counter.log();
            Ok(resp)
        }
        Err(_) => {
            let status = Status::internal(format!(
                "Failed to parse response: {text}\nuri: {uri}\nquery: {body}"
            ));
            counter.log_error(&status);
            Err(status)
        }
    }
}

const IGDB_SERVICE_URL: &str = "https://api.igdb.com/v4";
