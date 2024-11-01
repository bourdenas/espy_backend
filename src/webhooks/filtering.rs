use std::collections::HashSet;

use tracing::warn;

use crate::documents::{GameCategory, GameEntry, GameStatus, Notable, WebsiteAuthority};

pub struct GameFilter {
    companies: HashSet<String>,
    collections: HashSet<String>,
}

impl GameFilter {
    pub fn new(notable: Notable) -> Self {
        Self {
            companies: HashSet::<String>::from_iter(notable.legacy_companies.into_iter()),
            collections: HashSet::<String>::from_iter(notable.collections.into_iter()),
        }
    }

    pub fn apply(&self, game: &GameEntry) -> bool {
        game.scores.metacritic.is_some()
            || is_popular(game)
            || is_popular_early_access(game)
            || is_hyped(game)
            || game.category.is_expansion()
            || is_remaster(game)
            || self.is_notable(game)
            || is_gog_classic(game)
    }

    pub fn explain(&self, game: &GameEntry) -> RejectionReason {
        if !game.is_released() {
            if game.scores.hype.unwrap_or_default() == 0 {
                RejectionReason::FutureReleaseNoHype
            } else {
                warn!(
                    "GameFilter failed to provide rejection explanation for unreleased '{}' ({}).",
                    &game.name, game.id,
                );
                RejectionReason::Unknown
            }
        } else if game.is_early_access() && !is_popular_early_access(game) {
            RejectionReason::EarlyAccessLowPopularity
        } else if !is_popular(game) {
            RejectionReason::NoScoreLowPopularity
        } else {
            warn!(
                "GameFilter failed to provide rejection explanation for '{}' ({}).",
                &game.name, game.id,
            );
            RejectionReason::Unknown
        }
    }

    pub fn is_notable(&self, game: &GameEntry) -> bool {
        game.developers
            .iter()
            .any(|c| self.companies.contains(&c.name))
            || game
                .collections
                .iter()
                .any(|c| self.collections.contains(&c.name))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RejectionReason {
    FutureReleaseNoHype,
    EarlyAccessLowPopularity,
    NoScoreLowPopularity,
    Unknown,
}

impl std::fmt::Display for RejectionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn is_popular(game: &GameEntry) -> bool {
    (game.release_year() > 2011 && game.scores.popularity.unwrap_or_default() >= 10000)
        || (game.release_year() <= 2011 && game.scores.popularity.unwrap_or_default() > 0)
}

fn is_popular_early_access(game: &GameEntry) -> bool {
    game.is_early_access() && game.scores.popularity.unwrap_or_default() >= 5000
}

/// Returns true if game is/was hyped and is either future or recently released.
/// The intention is to prevent games from disappearing as soon as they release
/// if they don't get popular fast.
fn is_hyped(game: &GameEntry) -> bool {
    game.scores.hype.unwrap_or_default() > 0
        && (game.release_year() >= 2023 || game.release_date == 0)
        && !matches!(
            game.status,
            GameStatus::Cancelled | GameStatus::Alpha | GameStatus::Beta
        )
}

fn is_remaster(game: &GameEntry) -> bool {
    match game.category {
        GameCategory::Remake | GameCategory::Remaster => true,
        _ => false,
    }
}

fn is_gog_classic(game: &GameEntry) -> bool {
    game.release_year() < 2000
        && game
            .websites
            .iter()
            .any(|website| matches!(website.authority, WebsiteAuthority::Gog))
}
