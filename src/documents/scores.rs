use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::api::{IgdbGame, MetacriticData};

use super::{GogData, SteamData, WikipediaData};

#[derive(Eq, PartialEq, Serialize, Deserialize, Default, Clone, Debug)]
pub struct ScoresDoc {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub scores: Scores,
}

#[derive(Eq, PartialEq, Serialize, Deserialize, Default, Clone, Debug)]
pub struct Scores {
    // Thumbs up percentage from Steam.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbs: Option<u64>,

    // Popularity measured as total reviews on Steam.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popularity: Option<u64>,

    // Hype measured by IGDB follows and hypes for games that are not released yet.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hype: Option<u64>,

    // Aggregator score Metacritic or GameRankings.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metacritic: Option<u64>,

    // Metacritic score sourced either from Steam or IGDB.
    #[serde(default)]
    #[serde(skip_serializing_if = "MetacrtitcSource::is_metacritic")]
    pub metacritic_source: MetacrtitcSource,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub espy_score: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(skip_deserializing)]
    pub espy_tier: Option<EspyTier>,
}

impl Ord for Scores {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.espy_score
            .cmp(&other.espy_score)
            .then(self.popularity.cmp(&other.popularity))
            .then(self.thumbs.cmp(&other.thumbs))
            .then(self.hype.cmp(&other.hype))
    }
}

impl PartialOrd for Scores {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Scores {
    pub fn add_metacritic(&mut self, metacritic: MetacriticData, release_date: i64) {
        self.metacritic = Some(metacritic.score);
        self.metacritic_source = MetacrtitcSource::Metacritic;
        self.espy_score = if is_classic(release_date) {
            self.metacritic
        } else {
            match self.metacritic {
                Some(score) => {
                    let multiplier = match metacritic.review_count {
                        count if count >= 20 => 1.0,
                        count if count >= 10 => 0.9,
                        _ => 0.75,
                    };
                    Some((score as f64 * multiplier).round() as u64)
                }
                None => None,
            }
        };
        self.espy_tier = EspyTier::create(&self);
    }

    pub fn add_wikipedia(&mut self, wikipedia: WikipediaData) {
        if wikipedia.score.is_some() {
            self.metacritic_source = MetacrtitcSource::Wikipedia;
            self.metacritic = wikipedia.score;
            self.espy_score = wikipedia.score;
        }
    }

    pub fn add_gog(&mut self, gog_data: &GogData) {
        if self.metacritic.is_some() {
            return;
        }

        if let Some(score) = gog_data.critic_score {
            self.metacritic = Some(score);
            self.metacritic_source = MetacrtitcSource::Gog;
            self.espy_score = Some(score);
        }
    }

    pub fn add_steam(&mut self, steam_data: &SteamData, release_date: i64) {
        if let Some(score) = &steam_data.score {
            if score.review_score > 0 {
                self.thumbs = Some(score.review_score);
            }
        }

        if let Some(rec) = &steam_data.recommendations {
            self.popularity = match rec.total {
                0 => None,
                _ => Some(rec.total),
            };
        }

        if self.metacritic.is_none() {
            if let Some(metacritic) = &steam_data.metacritic {
                self.metacritic = Some(metacritic.score);
                self.metacritic_source = MetacrtitcSource::Steam;
            }

            self.espy_score = if is_classic(release_date) {
                self.metacritic
            } else {
                match self.metacritic {
                    Some(score) => {
                        let pop_multiplier = match self.popularity {
                            Some(pop) if pop >= 5000 => 1.0,
                            Some(pop) if pop >= 3000 => 0.9,
                            Some(pop) if pop >= 1000 => 0.75,
                            _ => 0.5,
                        };
                        Some((score as f64 * pop_multiplier).round() as u64)
                    }
                    None => None,
                }
            };
            self.espy_tier = EspyTier::create(&self);
        }
    }

    pub fn add_igdb(&mut self, igdb_game: &IgdbGame) {
        self.hype = igdb_game.hypes;
    }
}

#[derive(Eq, PartialEq, Serialize, Deserialize, Default, Clone, Debug)]
pub enum MetacrtitcSource {
    #[default]
    Metacritic,
    Wikipedia,
    Steam,
    Gog,
}

impl MetacrtitcSource {
    fn is_metacritic(&self) -> bool {
        matches!(self, MetacrtitcSource::Metacritic)
    }
}

// Returns true if game was released before 2011.
fn is_classic(release_date: i64) -> bool {
    const _41_YEARS: Duration = Duration::from_secs(41 * 365 * 24 * 60 * 60);
    let y2011 = UNIX_EPOCH + _41_YEARS;
    let release = UNIX_EPOCH + Duration::from_secs(release_date as u64);
    release < SystemTime::from(y2011)
}

#[derive(Eq, PartialEq, Serialize, Deserialize, Clone, Debug)]
pub enum EspyTier {
    Masterpiece = 95,
    Excellent = 90,
    Great = 80,
    Good = 70,
    Mixed = 60,
    Bad = 0,
    Unknown,
}

impl EspyTier {
    pub fn create(scores: &Scores) -> Option<Self> {
        match scores.espy_score {
            Some(x) if x >= Self::Masterpiece as u64 => Some(Self::Masterpiece),
            Some(x) if x >= Self::Excellent as u64 => Some(Self::Excellent),
            Some(x) if x >= Self::Great as u64 => Some(Self::Great),
            Some(x) if x >= Self::Good as u64 => Some(Self::Good),
            Some(x) if x > Self::Mixed as u64 => Some(Self::Mixed),
            Some(x) if x > Self::Bad as u64 => Some(Self::Bad),
            _ => None,
        }
    }
}
