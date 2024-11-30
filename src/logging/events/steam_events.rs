use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{log_event, logging::LogEvent};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct SteamEvent {
    api: SteamApi,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
enum SteamApi {
    GetOwnedGames { steam_id: String, game_count: usize },
    GetAppDetails { appid: String, name: String },
    GetAppScore { appid: String },
    ScrapeAppPage { url: String },
}

impl SteamEvent {
    pub fn get_owned_games(steam_id: &str, game_count: usize, errors: Vec<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            api: SteamApi::GetOwnedGames {
                steam_id: steam_id.to_owned(),
                game_count,
            },
            errors,
        }));
    }

    pub fn get_app_details(appid: String, name: String, errors: Vec<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            api: SteamApi::GetAppDetails { appid, name },
            errors,
        }));
    }

    pub fn get_app_score(appid: String, errors: Vec<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            api: SteamApi::GetAppScore { appid },
            errors,
        }));
    }

    pub fn scrape_app_page(url: String, errors: Vec<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            api: SteamApi::ScrapeAppPage { url },
            errors,
        }));
    }
}
