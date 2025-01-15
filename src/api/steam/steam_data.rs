use crate::{documents::SteamData, util::rate_limiter::RateLimiter, Status};
use std::time::Duration;
use tracing::{instrument, trace_span, warn, Instrument};

use super::{steam_scrape::SteamScrape, SteamApi};

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
    pub async fn retrieve_all_data(&self, steam_appid: &str) -> Result<SteamData, Status> {
        let scrape_handle = {
            let steam_appid = steam_appid.to_string();
            tokio::spawn(
                async move { SteamScrape::scrape(&steam_appid).await }
                    .instrument(trace_span!("spawn_steam_scrape")),
            )
        };

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

        match scrape_handle.await {
            Ok(result) => match result {
                Ok(steam_scrape_data) => steam_data.user_tags = steam_scrape_data.user_tags,
                Err(status) => warn!("{status}"),
            },
            Err(status) => warn!("{status}"),
        }

        Ok(steam_data)
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn retrieve_digest_data(&self, steam_appid: &str) -> Result<SteamData, Status> {
        self.qps.wait();
        let score = match SteamApi::get_app_score(steam_appid).await {
            Ok(result) => Some(result),
            Err(status) => {
                warn!("get_app_score(): {status}");
                None
            }
        };

        self.qps.wait();
        let mut steam_data = SteamApi::get_app_details(steam_appid).await?;
        steam_data.score = score;

        Ok(steam_data)
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn retrieve_expanded_data(&self, steam_data: &mut SteamData) -> Result<(), Status> {
        let scrape_handle = {
            let steam_appid = steam_data.steam_appid.to_string();
            tokio::spawn(
                async move { SteamScrape::scrape(&steam_appid).await }
                    .instrument(trace_span!("spawn_steam_scrape")),
            )
        };

        self.qps.wait();
        let news = match SteamApi::get_app_news(&steam_data.steam_appid.to_string()).await {
            Ok(result) => result,
            Err(status) => {
                warn!("get_app_news(): {status}");
                vec![]
            }
        };

        steam_data.news = news;

        match scrape_handle.await {
            Ok(result) => match result {
                Ok(steam_scrape_data) => steam_data.user_tags = steam_scrape_data.user_tags,
                Err(status) => warn!("{status}"),
            },
            Err(status) => warn!("{status}"),
        }

        Ok(())
    }
}
