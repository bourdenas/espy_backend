use crate::{documents::SteamData, util::rate_limiter::RateLimiter, Status};
use std::time::Duration;
use tracing::{instrument, warn};

use super::SteamApi;

pub struct SteamDataApi {
    qps: RateLimiter,
}

impl SteamDataApi {
    pub fn new() -> Self {
        SteamDataApi {
            qps: RateLimiter::new(200, Duration::from_secs(60), 7),
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn retrieve_steam_data(&self, steam_appid: &str) -> Result<SteamData, Status> {
        self.qps.wait();
        let score = match SteamApi::get_app_score(steam_appid).await {
            Ok(result) => Some(result),
            Err(status) => {
                warn!("get_app_score(): {status}");
                None
            }
        };

        self.qps.wait();
        let news = match SteamApi::get_app_news(steam_appid).await {
            Ok(result) => result,
            Err(status) => {
                warn!("get_app_news(): {status}");
                vec![]
            }
        };

        self.qps.wait();
        let mut steam_data = SteamApi::get_app_details(steam_appid).await?;
        steam_data.score = score;
        steam_data.news = news;

        Ok(steam_data)
    }
}
