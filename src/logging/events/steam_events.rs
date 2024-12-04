use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{log_event, logging::LogEvent};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct SteamEvent {
    method: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
enum SteamApi {
    GetOwnedGames { steam_id: String, game_count: usize },
    GetAppDetails { appid: String, name: String },
    GetAppScore { appid: String },
    ScrapeAppPage { url: String },
}

impl SteamEvent {
    pub fn get_owned_games(steam_id: &str, error: Option<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            method: "get_owned_games".to_owned(),
            id: Some(steam_id.to_owned()),
            url: None,
            error,
        }));
    }

    pub fn get_app_details(appid: String, error: Option<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            method: "get_app_details".to_owned(),
            id: Some(appid.to_owned()),
            url: None,
            error,
        }));
    }

    pub fn get_app_score(appid: String, error: Option<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            method: "get_app_score".to_owned(),
            id: Some(appid.to_owned()),
            url: None,
            error,
        }));
    }

    pub fn scrape_app_page(url: String, error: Option<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            method: "scrape_app_page".to_owned(),
            id: None,
            url: Some(url),
            error,
        }));
    }
}
