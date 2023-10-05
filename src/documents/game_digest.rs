use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{GameCategory, GameEntry};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct GameDigest {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub category: GameCategory,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<i64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub franchises: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub developers: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub publishers: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,
}

impl GameDigest {
    pub fn short_digest(game_entry: &GameEntry) -> Self {
        GameDigest {
            id: game_entry.id,
            name: game_entry.name.clone(),
            category: game_entry.category,

            cover: match &game_entry.cover {
                Some(cover) => Some(cover.image_id.clone()),
                None => None,
            },

            release_date: match game_entry.release_date {
                Some(date) => Some(date),
                None => game_entry.igdb_game.first_release_date,
            },
            rating: match game_entry.score {
                Some(score) => Some(score as f64),
                None => game_entry.igdb_game.aggregated_rating,
            },

            parent_id: match &game_entry.parent {
                Some(parent) => Some(parent.id),
                None => None,
            },
            ..Default::default()
        }
    }
}

impl From<GameEntry> for GameDigest {
    fn from(game_entry: GameEntry) -> Self {
        GameDigest {
            id: game_entry.id,
            name: game_entry.name,
            category: game_entry.category,

            cover: match game_entry.cover {
                Some(cover) => Some(cover.image_id),
                None => None,
            },

            release_date: match game_entry.release_date {
                Some(date) => Some(date),
                None => game_entry.igdb_game.first_release_date,
            },
            rating: match game_entry.score {
                Some(score) => Some(score as f64),
                None => game_entry.igdb_game.aggregated_rating,
            },

            parent_id: match game_entry.parent {
                Some(parent) => Some(parent.id),
                None => None,
            },

            collections: game_entry
                .collections
                .into_iter()
                .map(|collection| collection.name)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect(),

            franchises: game_entry
                .franchises
                .into_iter()
                .map(|franchise| franchise.name)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect(),

            developers: game_entry
                .developers
                .into_iter()
                .map(|company| company.name)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect(),

            publishers: game_entry
                .publishers
                .into_iter()
                .map(|company| company.name)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect(),

            genres: extract_genres(&game_entry.genres),
        }
    }
}

fn extract_genres(igdb_genres: &Vec<String>) -> Vec<String> {
    igdb_genres
        .iter()
        .filter_map(|igdb_genre| match GENRES.get(&igdb_genre) {
            Some(genre) => Some((*genre).to_owned()),
            None => None,
        })
        .collect()
}

use phf::phf_map;

static GENRES: phf::Map<&'static str, &'static str> = phf_map! {
    "Point-and-click" => "Adventure",
    "Adventure" => "Adventure",
    "Pinball" => "Arcade",
    "Arcade" => "Arcade",
    "Fighting" => "Arcade",
    "Card & Board Game" => "Arcade",
    "MOBA" => "Online",
    "Platform" => "Platformer",
    "Role-playing (RPG)" => "RPG",
    "Shooter" => "Shooter",
    "Racing" => "Simulator",
    "Simulator" => "Simulator",
    "Sport" => "Simulator",
    "Real Time Strategy (RTS)" => "Strategy",
    "Strategy" => "Strategy",
    "Turn-based strategy (TBS)" => "Strategy",
    "Tactical" => "Strategy",
};
