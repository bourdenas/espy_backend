use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{log_event, logging::LogEvent};

#[derive(Serialize, Deserialize, Valuable, Default, Debug)]
pub struct SteamEvent {
    pub get_owned_games: Option<GetOwnedGames>,
    pub get_app_details: Option<GetAppDetails>,
    pub get_app_score: Option<GetAppScore>,
    pub scrape_app_page: Option<ScrapeAppPage>,
}

#[derive(Serialize, Deserialize, Valuable, Default, Debug)]
pub struct GetOwnedGames {
    steam_id: String,
    game_count: usize,
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Valuable, Default, Debug)]
pub struct GetAppDetails {
    appid: String,
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Valuable, Default, Debug)]
pub struct GetAppScore {
    appid: String,
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Valuable, Default, Debug)]
pub struct ScrapeAppPage {
    url: String,
    error: Option<String>,
}

impl SteamEvent {
    pub fn get_owned_games(steam_id: &str, game_count: usize, error: Option<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            get_owned_games: Some(GetOwnedGames {
                steam_id: steam_id.to_owned(),
                game_count,
                error,
            }),
            ..Default::default()
        }));
    }

    pub fn get_app_details(appid: String, error: Option<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            get_app_details: Some(GetAppDetails { appid, error }),
            ..Default::default()
        }));
    }

    pub fn get_app_score(appid: String, error: Option<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            get_app_score: Some(GetAppScore { appid, error }),
            ..Default::default()
        }));
    }

    pub fn scrape_app_page(url: String, error: Option<String>) {
        log_event!(LogEvent::Steam(SteamEvent {
            scrape_app_page: Some(ScrapeAppPage { url, error }),
            ..Default::default()
        }));
    }
}
