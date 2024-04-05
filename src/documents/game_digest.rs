use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{GameCategory, GameEntry, GameStatus, IgdbGenre, Scores};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct GameDigest {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub category: GameCategory,

    #[serde(default)]
    pub status: GameStatus,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<i64>,

    #[serde(default)]
    pub scores: Scores,

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
    pub igdb_genres: Vec<IgdbGenre>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

impl From<GameEntry> for GameDigest {
    fn from(game_entry: GameEntry) -> Self {
        GameDigest {
            id: game_entry.id,
            name: game_entry.name,
            category: game_entry.category,
            status: game_entry.status,

            cover: match game_entry.cover {
                Some(cover) => Some(cover.image_id),
                None => None,
            },

            release_date: match game_entry.release_date {
                0 => None,
                x => Some(x),
            },
            scores: game_entry.scores.clone(),

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

            igdb_genres: game_entry.igdb_genres,
            genres: match &game_entry.steam_data {
                Some(steam_data) => steam_data
                    .genres
                    .iter()
                    .map(|genre| genre.description.clone())
                    .collect(),
                None => vec![],
            },
            keywords: match game_entry.steam_data {
                Some(steam_data) => steam_data.user_tags,
                None => vec![],
            },
        }
    }
}
