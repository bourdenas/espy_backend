use crate::{
    api::SteamApi,
    documents::{self, GameEntry},
    logging::SteamCounters,
    util::rate_limiter::RateLimiter,
    Status,
};
use chrono::NaiveDateTime;
use std::time::Duration;
use tracing::instrument;

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
            SteamCounters::missing_id(&game_entry);
            return Ok(());
        }

        SteamCounters::fetch(&game_entry);

        self.qps.wait();
        let score = match SteamApi::get_app_score(steam_appid.unwrap()).await {
            Ok(result) => Some(result),
            Err(status) => {
                SteamCounters::fetch_score_fail(&game_entry, &status);
                None
            }
        };
        self.qps.wait();
        game_entry.steam_data = match SteamApi::get_app_details(steam_appid.unwrap()).await {
            Ok(mut result) => {
                result.score = score;
                Some(result)
            }
            Err(status) => {
                SteamCounters::fetch_appdetails_fail(&game_entry, &status);
                return Err(Status::new(
                    &format!("Failed to retrieve Steam data for '{}'", game_entry.name),
                    status,
                ));
            }
        };

        game_entry.release_date = match &game_entry.steam_data.as_ref().unwrap().release_date {
            Some(date) => match NaiveDateTime::parse_from_str(
                &format!("{} 12:00:00", &date.date),
                "%e %b, %Y %H:%M:%S",
            ) {
                Ok(date) => Some(date.timestamp()),
                Err(status) => {
                    SteamCounters::date_parsing_fail(
                        &game_entry,
                        &Status::internal(format!("{status}")),
                    );
                    None
                }
            },
            None => None,
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
