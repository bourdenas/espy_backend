use crate::{documents::SteamData, util::rate_limiter::RateLimiter, Status};
use std::time::Duration;
use tracing::instrument;

use super::SteamApi;

pub struct SteamDataApi {
    qps: RateLimiter,
}

impl SteamDataApi {
    pub fn new() -> Self {
        SteamDataApi {
            qps: RateLimiter::new(200, Duration::from_secs(5 * 60), 7),
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn retrieve_steam_data(&self, steam_appid: &str) -> Result<SteamData, Status> {
        self.qps.wait();
        let score = match SteamApi::get_app_score(steam_appid).await {
            Ok(result) => Some(result),
            Err(_) => None,
        };

        self.qps.wait();
        let mut steam_data = SteamApi::get_app_details(steam_appid).await?;
        steam_data.score = score;

        Ok(steam_data)
    }
}
