use crate::api::IgdbGame;

use super::counters::*;

pub struct IgdbCounters;

impl IgdbCounters {
    pub fn igdb_resolve(igdb_game: &IgdbGame) {
        counter(
            "igdb_resolve",
            &format!("IGDB resolve: '{}' ({})", &igdb_game.name, &igdb_game.id),
        )
    }
}
