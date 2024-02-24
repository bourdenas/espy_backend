use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::api::{IgdbGame, MetacriticData};

use super::SteamData;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
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

    // Metacritic score sourced either from Steam or IGDB.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metacritic: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub espy_score: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(skip_deserializing)]
    pub espy_tier: Option<EspyTier>,
}

impl Scores {
    pub fn add_metacritic(&mut self, metacritic: MetacriticData) {
        self.metacritic = metacritic.score;
        self.espy_score = self.metacritic;
        self.espy_tier = EspyTier::create(&self);
    }

    pub fn add_steam(&mut self, steam_data: &SteamData, release_date: i64) {
        if let Some(score) = &steam_data.score {
            self.thumbs = Some(score.review_score);
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
            }
        }

        if !is_classic(release_date) {
            if let Some(score) = self.metacritic {
                let pop_multiplier = match self.popularity {
                    Some(pop) if pop >= 5000 => 1.0,
                    Some(pop) if pop >= 3000 => 0.9,
                    Some(pop) if pop >= 1000 => 0.75,
                    _ => 0.5,
                };
                self.espy_score = Some((score as f64 * pop_multiplier).round() as u64);
            }
        } else {
            self.espy_score = self.metacritic;
        }
        self.espy_tier = EspyTier::create(&self);
    }

    pub fn add_igdb(&mut self, igdb_game: &IgdbGame) {
        self.hype = igdb_game.hypes;
    }
}

fn is_classic(release_date: i64) -> bool {
    const _41_YEARS: Duration = Duration::from_secs(41 * 365 * 24 * 60 * 60);
    let y2011 = UNIX_EPOCH + _41_YEARS;
    let release = UNIX_EPOCH + Duration::from_secs(release_date as u64);
    release < SystemTime::from(y2011)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
