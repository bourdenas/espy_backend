use crate::{
    api::SteamApi,
    documents::{GameEntry, Scores},
    logging::SteamFetchCounter,
    util::rate_limiter::RateLimiter,
    Status,
};
use chrono::NaiveDateTime;
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
    pub async fn retrieve_steam_data(
        &self,
        steam_appid: &str,
        game_entry: &mut GameEntry,
    ) -> Result<(), Status> {
        let counter = SteamFetchCounter::new();

        self.qps.wait();
        let score = match SteamApi::get_app_score(steam_appid).await {
            Ok(result) => Some(result),
            Err(status) => {
                counter.log_warning("fetch_score_fail", game_entry, &status);
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
                counter.log_error(game_entry, &status);
                return Err(status);
            }
        };

        warn!("steam_data={:?}", steam_data);

        game_entry.release_date = match &steam_data.release_date {
            // TODO: Make parsing more resilient to location formatting.
            Some(date) => match NaiveDateTime::parse_from_str(
                &format!("{} 12:00:00", &date.date),
                "%b %e, %Y %H:%M:%S",
            ) {
                Ok(date) => Some(date.timestamp()),
                Err(status) => {
                    counter.log_warning(
                        "date_parsing_fail",
                        game_entry,
                        &Status::invalid_argument(format!(
                            "Invalid date format '{}': {status}",
                            &date.date
                        )),
                    );
                    game_entry.release_date
                }
            },
            None => game_entry.release_date,
        };
        game_entry.scores = Scores {
            tier: match &steam_data.score {
                Some(score) => match score.review_score_desc.as_str() {
                    "Overwhelmingly Positive" => Some(9),
                    "Very Positive" => Some(8),
                    "Positive" => Some(7),
                    "Mostly Positive" => Some(6),
                    "Mixed" => Some(5),
                    "Mostly Negative" => Some(4),
                    "Negative" => Some(3),
                    "Very Negative" => Some(2),
                    "Overwhelmingly Negative" => Some(1),
                    _ => None,
                },
                None => None,
            },
            thumbs: match &steam_data.score {
                Some(score) => Some(score.review_score),
                None => game_entry.scores.thumbs,
            },
            popularity: match &steam_data.score {
                Some(score) => match score.total_reviews {
                    0 => game_entry.scores.popularity,
                    _ => Some(score.total_reviews),
                },
                None => game_entry.scores.popularity,
            },
            metacritic: match &steam_data.metacritic {
                Some(metacrtic) => Some(metacrtic.score),
                None => game_entry.scores.metacritic,
            },
        };
        game_entry.steam_data = Some(steam_data);

        counter.log(&game_entry);
        Ok(())
    }
}
