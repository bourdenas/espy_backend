use crate::api::IgdbGame;

pub struct IgdbPrefilter;

impl IgdbPrefilter {
    pub fn filter(igdb_game: &IgdbGame) -> bool {
        igdb_game.is_pc_game()
            && igdb_game.is_main_category()
            && (igdb_game.follows.unwrap_or_default() > 0
                || igdb_game.hypes.unwrap_or_default() > 0
                || igdb_game.aggregated_rating.unwrap_or_default() > 0.0)
    }

    pub fn explain(igdb_game: &IgdbGame) -> PrefilterRejectionReason {
        if !igdb_game.is_pc_game() {
            PrefilterRejectionReason::NotPcGame
        } else if !igdb_game.is_main_category() {
            PrefilterRejectionReason::NotMainCategory
        } else if igdb_game.follows.unwrap_or_default() == 0
            && igdb_game.hypes.unwrap_or_default() == 0
            && igdb_game.aggregated_rating.is_none()
        {
            PrefilterRejectionReason::NoUserMetrics
        } else {
            PrefilterRejectionReason::Unknown
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PrefilterRejectionReason {
    NotPcGame,
    NotMainCategory,
    NoUserMetrics,
    Unknown,
}

impl std::fmt::Display for PrefilterRejectionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
