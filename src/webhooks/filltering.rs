use std::collections::HashSet;

use crate::{
    api::IgdbGame,
    documents::{
        EspyGenre, GameCategory, GameEntry, GameStatus, Notable, SteamData, WebsiteAuthority,
    },
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum GameEntryClass {
    Main,
    Expansion,
    EarlyAccess,
    Indie,
    Casual,
    Debug,
    Ignore,
}

pub struct GameEntryClassifier {
    companies: HashSet<String>,
    collections: HashSet<String>,
}

impl GameEntryClassifier {
    pub fn new(notable: Notable) -> Self {
        Self {
            companies: HashSet::<String>::from_iter(notable.legacy_companies.into_iter()),
            collections: HashSet::<String>::from_iter(notable.collections.into_iter()),
        }
    }

    pub fn pre_filter(&self, igdb_game: &IgdbGame) -> bool {
        igdb_game.is_pc_game()
            && igdb_game.is_main_category()
            && (igdb_game.follows.unwrap_or_default() > 0
                || igdb_game.hypes.unwrap_or_default() > 0
                || igdb_game.aggregated_rating.unwrap_or_default() > 0.0)
    }

    pub fn filter(&self, game: &GameEntry) -> bool {
        !matches!(self.classify(game), GameEntryClass::Ignore)
    }

    pub fn classify(&self, game: &GameEntry) -> GameEntryClass {
        if !game.is_released() {
            match is_hyped_tbd(&game) {
                true => GameEntryClass::Main,
                false => GameEntryClass::Ignore,
            }
        } else if is_popular_early_access(&game) {
            GameEntryClass::EarlyAccess
        } else if is_expansion(&game) && game.scores.metacritic.is_none() {
            GameEntryClass::Expansion
        } else if is_indie(&game) {
            if game.scores.metacritic.is_some() || is_popular(game) {
                match is_casual(game) {
                    true => GameEntryClass::Casual,
                    false => GameEntryClass::Indie,
                }
            } else {
                GameEntryClass::Ignore
            }
        } else if game.scores.metacritic.is_some()
            || is_popular(game)
            || is_remaster(game)
            || is_notable(game, &self.companies, &self.collections)
            || is_gog_classic(&game)
        {
            match is_casual(game) {
                true => GameEntryClass::Casual,
                false => GameEntryClass::Main,
            }
        } else {
            GameEntryClass::Ignore
        }
    }

    pub fn explain(&self, game: &GameEntry) -> RejectionReason {
        if !game.is_released() {
            if game.scores.hype.unwrap_or_default() == 0 {
                RejectionReason::FutureReleaseNoHype
            } else if game.scores.thumbs.is_some() {
                RejectionReason::FutureReleaseWithThumbsUp
            } else if is_casual(game) {
                RejectionReason::FutureReleaseCasual
            } else {
                RejectionReason::Unknown
            }
        } else if is_early_access(game) && !is_popular_early_access(game) {
            RejectionReason::EarlyAccessLowPopularity
        } else if is_popular(game) {
            RejectionReason::NoScoreLowPopularity
        } else {
            RejectionReason::Unknown
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RejectionReason {
    FutureReleaseNoHype,
    FutureReleaseWithThumbsUp,
    FutureReleaseCasual,
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
    is_early_access(game) && game.scores.popularity.unwrap_or_default() >= 5000
}

fn is_hyped_tbd(game: &GameEntry) -> bool {
    !matches!(
        game.status,
        GameStatus::Cancelled | GameStatus::Alpha | GameStatus::Beta
    ) && game.scores.hype.unwrap_or_default() > 0
        && game.scores.thumbs.is_none()
        && !is_casual(&game)
}

fn is_indie(game: &GameEntry) -> bool {
    game.release_year() > 2007
        && game
            .espy_genres
            .iter()
            .any(|genre| matches!(genre, EspyGenre::Indie))
}

fn is_early_access(game: &GameEntry) -> bool {
    game.release_year() > 2018
        && matches!(game.status, GameStatus::EarlyAccess)
        && game.scores.metacritic.is_none()
}

fn is_remaster(game: &GameEntry) -> bool {
    match game.category {
        GameCategory::Remake | GameCategory::Remaster => true,
        _ => false,
    }
}

fn is_expansion(game: &GameEntry) -> bool {
    matches!(
        game.category,
        GameCategory::Expansion | GameCategory::StandaloneExpansion
    )
}

fn is_casual(game: &GameEntry) -> bool {
    game.steam_data
        .as_ref()
        .unwrap_or(&SteamData::default())
        .genres
        .iter()
        .any(|genre| genre.description == "Casual")
}

fn is_gog_classic(game: &GameEntry) -> bool {
    game.release_year() < 2000
        && game
            .websites
            .iter()
            .any(|website| matches!(website.authority, WebsiteAuthority::Gog))
}

fn is_notable(
    game: &GameEntry,
    companies: &HashSet<String>,
    collections: &HashSet<String>,
) -> bool {
    game.developers.iter().any(|c| companies.contains(&c.name))
        || game
            .collections
            .iter()
            .any(|c| collections.contains(&c.name))
}