use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    documents::{GameCategory, GameEntry, GameStatus, Notable, WebsiteAuthority},
    logging::RejectEvent,
};

pub struct GameFilter {
    companies: HashSet<String>,
    collections: HashSet<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RejectionReason {
    pub reason: Reason,

    pub category: GameCategory,
    pub status: GameStatus,

    pub popularity: u64,
    pub hype: u64,
    pub year: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Reason {
    NoScoreLowPopularity,
    FutureReleaseNoHype,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RejectionException {
    pub exception: Exception,

    pub category: GameCategory,
    pub status: GameStatus,

    pub popularity: u64,
    pub hype: u64,
    pub year: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Exception {
    Expansion,
    Remaster,
    Notable(NotableFor),
    GogClassic,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NotableFor {
    Developer(String),
    Collection(String),
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
            || is_hyped(game)
            || self.exception(game)
    }

    fn exception(&self, game: &GameEntry) -> bool {
        let exception = if game.category.is_expansion() {
            Some(Exception::Expansion)
        } else if game.category.is_remaster() {
            Some(Exception::Remaster)
        } else if let Some(notable) = self.is_notable(game) {
            Some(Exception::Notable(notable))
        } else if is_gog_classic(game) {
            Some(Exception::GogClassic)
        } else {
            None
        };

        if let Some(exception) = exception {
            RejectEvent::exception(RejectionException {
                exception,
                category: game.category,
                status: game.status,
                popularity: game.scores.popularity.unwrap_or_default(),
                hype: game.scores.hype.unwrap_or_default(),
                year: game.release_year(),
            });
            true
        } else {
            false
        }
    }

    pub fn explain(&self, game: &GameEntry) -> RejectionReason {
        let reason = if !game.is_released() && game.scores.hype.unwrap_or_default() == 0 {
            Reason::FutureReleaseNoHype
        } else if !is_popular(game) {
            Reason::NoScoreLowPopularity
        } else {
            warn!(
                "GameFilter failed to provide rejection explanation for '{}' ({}).",
                &game.name, game.id,
            );
            Reason::Unknown
        };

        RejectionReason {
            reason,
            category: game.category,
            status: game.status,
            popularity: game.scores.popularity.unwrap_or_default(),
            hype: game.scores.hype.unwrap_or_default(),
            year: game.release_year(),
        }
    }

    pub fn is_notable(&self, game: &GameEntry) -> Option<NotableFor> {
        if let Some(dev) = game
            .developers
            .iter()
            .find(|c| self.companies.contains(&c.name))
        {
            Some(NotableFor::Developer(dev.name.clone()))
        } else if let Some(col) = game
            .collections
            .iter()
            .find(|c| self.collections.contains(&c.name))
        {
            Some(NotableFor::Collection(col.name.clone()))
        } else {
            None
        }
    }
}

fn is_popular(game: &GameEntry) -> bool {
    game.scores.popularity.unwrap_or_default() >= 10000
        || (game.is_early_access() && game.scores.popularity.unwrap_or_default() >= 5000)
        || (game.release_year() <= 2011 && game.scores.popularity.unwrap_or_default() > 0)
}

/// Returns true if game is/was hyped and is either future or recently released.
/// The intention is to prevent games from disappearing as soon as they release
/// if they don't get popular fast.
fn is_hyped(game: &GameEntry) -> bool {
    game.scores.hype.unwrap_or_default() > 0
        && (game.release_year() >= 2024 || game.release_date == 0)
        && !matches!(
            game.status,
            GameStatus::Cancelled | GameStatus::Alpha | GameStatus::Beta
        )
}

fn is_gog_classic(game: &GameEntry) -> bool {
    game.release_year() < 2000
        && game
            .websites
            .iter()
            .any(|website| matches!(website.authority, WebsiteAuthority::Gog))
}
