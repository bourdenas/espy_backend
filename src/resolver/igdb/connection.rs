use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{util::rate_limiter::RateLimiter, Status};

#[derive(Debug)]
pub struct IgdbConnection {
    pub client_id: String,
    pub oauth_token: String,
    pub qps: RateLimiter,
}

impl IgdbConnection {
    /// Authenticate with twtich/igdb OAuth2 server and retrieve session token.
    /// Authentication is valid for the lifetime of this instane or until the
    /// retrieved token expires.
    pub async fn new(client_id: &str, secret: &str) -> Result<Self, Status> {
        let uri = format!(
            "{TWITCH_OAUTH_URL}?client_id={client_id}&client_secret={secret}&grant_type=client_credentials");

        let resp = reqwest::Client::new()
            .post(&uri)
            .send()
            .await?
            .json::<TwitchOAuthResponse>()
            .await?;

        Ok(IgdbConnection {
            client_id: client_id.to_string(),
            oauth_token: resp.access_token,
            qps: RateLimiter::new(4, Duration::from_secs(1), 7),
        })
    }
}

pub const TWITCH_OAUTH_URL: &str = "https://id.twitch.tv/oauth2/token";

#[derive(Debug, Serialize, Deserialize)]
struct TwitchOAuthResponse {
    access_token: String,
    expires_in: i32,
}
