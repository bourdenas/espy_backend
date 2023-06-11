use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{GameCategory, GameEntry};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct GameDigest {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<i64>,

    #[serde(default)]
    pub category: GameCategory,

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

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

impl GameDigest {
    pub fn short_digest(game_entry: &GameEntry) -> Self {
        GameDigest {
            id: game_entry.id,
            name: game_entry.name.clone(),
            cover: match &game_entry.cover {
                Some(cover) => Some(cover.image_id.clone()),
                None => None,
            },
            release_date: game_entry.release_date,
            category: game_entry.category,
            rating: game_entry.igdb_rating,
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

            cover: match game_entry.cover {
                Some(cover) => Some(cover.image_id),
                None => None,
            },

            release_date: game_entry.release_date,
            category: game_entry.category,
            rating: game_entry.igdb_rating,

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

            genres: match game_entry.steam_data {
                Some(steam_data) => steam_data
                    .genres
                    .into_iter()
                    .map(|genre| genre.description)
                    .collect(),
                None => game_entry.genres,
            },
            keywords: game_entry.keywords,
        }
    }
}
