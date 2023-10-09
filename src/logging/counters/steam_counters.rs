use std::time::SystemTime;

use tracing::info;

use crate::{documents::GameEntry, Status};

pub struct SteamFetchCounter {
    start: SystemTime,
}

impl SteamFetchCounter {
    pub fn new() -> Self {
        Self {
            start: SystemTime::now(),
        }
    }

    pub fn log(self, game_entry: &GameEntry) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = STEAM,
            counter.name = "fetch",
            counter.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "Steam fetch: {}",
            game_entry_description(game_entry),
        )
    }

    pub fn log_warning(&self, warning: &str, game_entry: &GameEntry, status: &Status) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = STEAM,
            counter.name = warning,
            counter.status = status.to_string(),
            "Steam warning '{warning}': {}",
            game_entry_description(game_entry),
        )
    }

    pub fn log_error(self, game_entry: &GameEntry, status: &Status) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = STEAM,
            counter.name = "fetch_fail",
            counter.status = status.to_string(),
            "Steam fetch fail: {}",
            game_entry_description(game_entry),
        )
    }
}

fn game_entry_description(game_entry: &GameEntry) -> String {
    format!(
        "'{}', igdb: {}, steam: {}",
        game_entry.name,
        game_entry.id,
        match &game_entry.steam_data {
            Some(steam_data) => steam_data.steam_appid.to_string(),
            None => "none".to_owned(),
        }
    )
}

const COUNTERS: &str = "counters";
const STEAM: &str = "steam";
