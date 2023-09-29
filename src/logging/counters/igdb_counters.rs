use crate::{api::IgdbGame, Status};

use super::counters::*;

pub struct IgdbCounters;

impl IgdbCounters {
    pub fn resolve(igdb_game: &IgdbGame) {
        counter(
            "igdb_resolve",
            &format!("IGDB resolve: '{}' ({})", &igdb_game.name, &igdb_game.id),
        )
    }

    pub fn request_fail(status: &Status) {
        error_counter("igdb_request_fail", &format!("IGDB request failed"), status)
    }

    pub fn response_parsing_fail(status: &Status) {
        error_counter(
            "igdb_response_parsing_fail",
            &format!("IGDB response parsing failed"),
            status,
        )
    }
}
