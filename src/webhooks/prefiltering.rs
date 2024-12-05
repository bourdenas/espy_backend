use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::documents::{GameCategory, GamePlatform, IgdbGame};

pub struct IgdbPrefilter;

impl IgdbPrefilter {
    pub fn filter(igdb_game: &IgdbGame) -> bool {
        igdb_game.is_pc_game()
            && igdb_game.is_watched_category()
            && (igdb_game.follows.unwrap_or_default() > 0
                || igdb_game.hypes.unwrap_or_default() > 0
                || igdb_game.aggregated_rating.unwrap_or_default() > 0.0)
    }

    pub fn explain(igdb_game: &IgdbGame) -> PrefilterRejectionReason {
        if !igdb_game.is_pc_game() {
            PrefilterRejectionReason::NotPcGame(
                igdb_game
                    .platforms
                    .iter()
                    .map(|id| GamePlatform::from(*id))
                    .collect(),
            )
        } else if !igdb_game.is_watched_category() {
            PrefilterRejectionReason::NotMainCategory(GameCategory::from(igdb_game.category))
        } else if igdb_game.follows.unwrap_or_default() == 0
            && igdb_game.hypes.unwrap_or_default() == 0
            && igdb_game.aggregated_rating.is_none()
        {
            PrefilterRejectionReason::NoUserMetrics
        } else {
            warn!(
                "Prefilter failed to provide rejection explanation for '{}' ({}).",
                &igdb_game.name, igdb_game.id,
            );
            PrefilterRejectionReason::Unknown
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PrefilterRejectionReason {
    NotPcGame(Vec<GamePlatform>),
    NotMainCategory(GameCategory),
    NoUserMetrics,
    Unknown,
}

impl std::fmt::Display for PrefilterRejectionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
