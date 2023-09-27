use serde::{Deserialize, Serialize};

use crate::api::IgdbGame;

use super::{GameDigest, SteamData};

/// Document type under 'users/{user_id}/games' that represents a game entry in
/// IGDB.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct GameEntry {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub category: GameCategory,

    #[serde(default)]
    pub status: GameStatus,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<i64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Image>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<CollectionDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub franchises: Vec<CollectionDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub developers: Vec<CompanyDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub publishers: Vec<CompanyDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub expansions: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dlcs: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remakes: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remasters: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<Image>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artwork: Vec<Image>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub websites: Vec<Website>,

    #[serde(default)]
    pub igdb_game: IgdbGame,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam_data: Option<SteamData>,
}

impl From<IgdbGame> for GameEntry {
    fn from(igdb_game: IgdbGame) -> Self {
        GameEntry {
            id: igdb_game.id,
            name: igdb_game.name.clone(),

            category: match igdb_game.version_parent {
                Some(_) => GameCategory::Version,
                None => GameCategory::from(igdb_game.category),
            },
            status: GameStatus::from(igdb_game.status),

            parent: match igdb_game.parent_game {
                Some(id) => Some(GameDigest {
                    id,
                    ..Default::default()
                }),
                None => match igdb_game.version_parent {
                    Some(id) => Some(GameDigest {
                        id,
                        ..Default::default()
                    }),
                    None => None,
                },
            },

            websites: vec![Website {
                url: igdb_game.url.clone(),
                authority: WebsiteAuthority::Igdb,
            }],

            igdb_game,

            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum GameCategory {
    Main,
    Dlc,
    Expansion,
    Bundle,
    StandaloneExpansion,
    Episode,
    Season,
    Remake,
    Remaster,
    Version,
    Ignore,
}

impl From<u64> for GameCategory {
    fn from(igdb_category: u64) -> Self {
        match igdb_category {
            0 => GameCategory::Main,
            1 => GameCategory::Dlc,
            2 => GameCategory::Expansion,
            3 => GameCategory::Bundle,
            4 => GameCategory::StandaloneExpansion,
            6 => GameCategory::Episode,
            7 => GameCategory::Season,
            8 => GameCategory::Remake,
            9 | 10 | 14 => GameCategory::Remaster,
            _ => GameCategory::Ignore,
        }
    }
}

impl Default for GameCategory {
    fn default() -> Self {
        GameCategory::Main
    }
}

impl std::fmt::Display for GameCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum GameStatus {
    Unknown,
    Released,
    Alpha,
    Beta,
    EarlyAccess,
    Offline,
    Cancelled,
    Rumored,
    Delisted,
}

impl From<u64> for GameStatus {
    fn from(igdb_category: u64) -> Self {
        match igdb_category {
            0 => GameStatus::Released,
            2 => GameStatus::Alpha,
            3 => GameStatus::Beta,
            4 => GameStatus::EarlyAccess,
            5 => GameStatus::Offline,
            6 => GameStatus::Cancelled,
            7 => GameStatus::Rumored,
            8 => GameStatus::Delisted,
            _ => GameStatus::Unknown,
        }
    }
}

impl Default for GameStatus {
    fn default() -> Self {
        GameStatus::Released
    }
}

impl std::fmt::Display for GameStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Image {
    pub image_id: String,

    #[serde(default)]
    pub height: i32,

    #[serde(default)]
    pub width: i32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CompanyDigest {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub slug: String,

    #[serde(default)]
    pub role: CompanyRole,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CompanyRole {
    Unknown = 0,
    Developer = 1,
    Publisher = 2,
    Porting = 3,
    Support = 4,
}

impl Default for CompanyRole {
    fn default() -> Self {
        CompanyRole::Unknown
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CollectionDigest {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub slug: String,

    pub igdb_type: CollectionType,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum CollectionType {
    Null = 0,
    Collection = 1,
    Franchise = 2,
}

impl Default for CollectionType {
    fn default() -> Self {
        CollectionType::Null
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Website {
    pub url: String,
    pub authority: WebsiteAuthority,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WebsiteAuthority {
    Null = 0,
    Official = 1,
    Wikipedia = 2,
    Igdb = 3,
    Gog = 4,
    Steam = 5,
    Egs = 6,
    Youtube = 7,
}

impl Default for WebsiteAuthority {
    fn default() -> Self {
        WebsiteAuthority::Null
    }
}
