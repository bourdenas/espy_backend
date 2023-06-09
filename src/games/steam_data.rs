use crate::{
    api::SteamApi,
    documents::{self, GameEntry},
    util::rate_limiter::RateLimiter,
    Status,
};
use std::time::Duration;
use tracing::{instrument, warn};

pub struct SteamDataApi {
    qps: RateLimiter,
}

impl SteamDataApi {
    pub fn new() -> Self {
        SteamDataApi {
            qps: RateLimiter::new(200, Duration::from_secs(5 * 60), 7),
        }
    }

    #[instrument(
        level = "trace",
        skip(self, game_entry),
        fields(game_entry = %game_entry.name),
    )]
    pub async fn retrieve_steam_data(&self, game_entry: &mut GameEntry) -> Result<(), Status> {
        let steam_appid = get_steam_appid(game_entry);

        if let None = steam_appid {
            warn!("Missing steam entry for '{}'", game_entry.name);
            return Ok(());
        }

        self.qps.wait();
        game_entry.steam_data = match SteamApi::get_app_details(steam_appid.unwrap()).await {
            Ok(result) => Some(result),
            Err(e) => {
                return Err(Status::new(
                    &format!("Failed to retrieve Steam data for '{}'", game_entry.name),
                    e,
                ));
            }
        };

        Ok(())
    }
}

fn get_steam_appid(game_entry: &GameEntry) -> Option<u64> {
    game_entry
        .websites
        .iter()
        .find_map(|website| match website.authority {
            documents::WebsiteAuthority::Steam => website
                .url
                .split("/")
                .collect::<Vec<_>>()
                .iter()
                .rev()
                .find_map(|s| s.parse().ok()),
            _ => None,
        })
}
