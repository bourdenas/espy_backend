use tracing::info;

use crate::{documents::GameEntry, Status};

pub fn counter(name: &str, description: &str) {
    info!(
        labels.log_type = "counters",
        labels.counter = name,
        description
    );
}

pub fn error_counter(name: &str, description: &str, status: &Status) {
    info!(
        labels.log_type = "counters",
        labels.counter_type = "error",
        labels.status = status.to_string(),
        labels.counter = name,
        description
    );
}

pub fn game_entry_description(game_entry: &GameEntry) -> String {
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
