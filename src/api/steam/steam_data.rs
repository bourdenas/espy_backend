use crate::{
    documents::SteamData, logging::SteamFetchCounter, util::rate_limiter::RateLimiter, Status,
};
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
        let counter = SteamFetchCounter::new();

        self.qps.wait();
        let score = match SteamApi::get_app_score(steam_appid).await {
            Ok(result) => Some(result),
            Err(status) => {
                counter.log_warning("fetch_score_fail", &status);
                None
            }
        };
        self.qps.wait();
        let steam_data = match SteamApi::get_app_details(steam_appid).await {
            Ok(mut steam_data) => {
                steam_data.score = score;
                steam_data
            }
            Err(status) => {
                counter.log_error(&status);
                return Err(status);
            }
        };

        counter.log();
        Ok(steam_data)
    }
}
