use crate::{documents::GameEntry, Status};

use super::counters::*;

pub struct SteamCounters;

impl SteamCounters {
    pub fn fetch(game_entry: &GameEntry) {
        counter(
            "steam_fetch",
            &format!("Steam fetch: {}", game_entry_description(game_entry)),
        )
    }

    pub fn missing_id(game_entry: &GameEntry) {
        counter(
            "steam_missing_id",
            &format!("Steam Id missing: {}", game_entry_description(game_entry)),
        )
    }

    pub fn fetch_score_fail(game_entry: &GameEntry, status: &Status) {
        error_counter(
            "steam_fetch_score_fail",
            &format!(
                "Steam fetch score fail: {}",
                game_entry_description(game_entry)
            ),
            status,
        )
    }

    pub fn fetch_appdetails_fail(game_entry: &GameEntry, status: &Status) {
        error_counter(
            "steam_fetch_appdetails_fail",
            &format!(
                "Steam fetch appdetails fail: {}",
                game_entry_description(game_entry)
            ),
            status,
        )
    }

    pub fn date_parsing_fail(game_entry: &GameEntry, status: &Status) {
        error_counter(
            "steam_date_parsing_fail",
            &format!(
                "Steam date parsing failed: {}",
                game_entry_description(game_entry)
            ),
            status,
        )
    }
}
