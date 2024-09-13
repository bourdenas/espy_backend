use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

use crate::api::IgdbGame;

use super::{EspyGenre, GameDigest, GogData, Scores, SteamData};

/// Document type under 'games' collection that represents an espy game entry.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct GameEntry {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub category: GameCategory,

    #[serde(default)]
    pub status: GameStatus,

    #[serde(default)]
    pub last_updated: i64,

    #[serde(default)]
    pub release_date: i64,

    #[serde(default)]
    pub scores: Scores,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Image>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub espy_genres: Vec<EspyGenre>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub igdb_genres: Vec<IgdbGenre>,

    // Keywords from IGDB.
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

    // If the GameEntry is a Bundle or Version `contents` includes the digests
    // of all individual entries it contains.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contents: Vec<GameDigest>,

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

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gog_data: Option<GogData>,
}

impl GameEntry {
    pub fn resolve_genres(&mut self) {
        self.igdb_genres = self
            .igdb_game
            .genres
            .iter()
            .filter_map(|igdb_genre_id| match GENRES_BY_ID.get(&igdb_genre_id) {
                Some(genre) => Some(*genre),
                None => None,
            })
            .collect();
    }

    pub fn get_steam_appid(&self) -> Option<String> {
        self.websites
            .iter()
            .find_map(|website| match website.authority {
                WebsiteAuthority::Steam => website
                    .url
                    .split("/")
                    .collect::<Vec<_>>()
                    .iter()
                    .rev()
                    .find_map(|s| Some(s.to_string())),
                _ => None,
            })
    }

    pub fn add_steam_data(&mut self, steam_data: SteamData) {
        self.scores.add_steam(&steam_data, self.release_date);
        self.steam_data = Some(steam_data);
    }

    pub fn add_gog_data(&mut self, gog_data: GogData) {
        self.scores.add_gog(&gog_data);
        self.gog_data = Some(gog_data);
    }

    pub fn update(&mut self, igdb_game: IgdbGame) {
        self.name = igdb_game.name.clone();
        self.category = Self::extract_category(&igdb_game);
        self.status = GameStatus::from(igdb_game.status);
        self.scores.add_igdb(&igdb_game);

        self.igdb_game = igdb_game;
    }

    pub fn release_year(&self) -> i32 {
        DateTime::from_timestamp(self.release_date, 0)
            .unwrap()
            .year()
    }

    pub fn is_released(&self) -> bool {
        self.release_date > 0 && self.release_date < Utc::now().naive_utc().and_utc().timestamp()
    }

    fn extract_category(igdb_game: &IgdbGame) -> GameCategory {
        match igdb_game.version_parent {
            Some(_) => GameCategory::Version,
            None => GameCategory::from(igdb_game.category),
        }
    }
}

impl From<IgdbGame> for GameEntry {
    fn from(igdb_game: IgdbGame) -> Self {
        GameEntry {
            id: igdb_game.id,
            name: igdb_game.name.clone(),

            category: GameEntry::extract_category(&igdb_game),
            status: GameStatus::from(igdb_game.status),

            release_date: match igdb_game.first_release_date {
                Some(timestamp) => timestamp,
                None => 0,
            },
            scores: {
                let mut scores = Scores::default();
                scores.add_igdb(&igdb_game);
                scores
            },

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
    Unknown,
}

impl GameCategory {
    pub fn is_main_category(&self) -> bool {
        matches!(
            self,
            GameCategory::Main
                | GameCategory::Expansion
                | GameCategory::StandaloneExpansion
                | GameCategory::Remake
                | GameCategory::Remaster
        )
    }

    pub fn is_expansion(&self) -> bool {
        matches!(
            self,
            GameCategory::Expansion | GameCategory::StandaloneExpansion
        )
    }
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
        GameCategory::Unknown
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
        GameStatus::Unknown
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
    // This is a normalized version of the name removing fluff. It is used to
    // combine variations of a company name, e.g. "Interplay" -> "Interplay
    // Entertainment" or its evolution over time, e.g. "LucasFilm" ->
    // "LucasArts".
    pub slug: String,

    #[serde(default)]
    pub role: CompanyRole,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum CompanyRole {
    Unknown,
    Developer,
    Publisher,
    DevPub,
    Porting,
    Support,
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

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum IgdbGenre {
    PointAndClick,
    Fighting,
    Shooter,
    Music,
    Platformer,
    Puzzle,
    Racing,
    RealTimeStrategy,
    RPG,
    Simulator,
    Sports,
    Strategy,
    TurnBasedStrategy,
    Tactical,
    HackAndSlash,
    Quiz,
    Pinball,
    Adventure,
    Indie,
    Arcade,
    VisualNovel,
    CardAndBoard,
    MOBA,
}

use phf::phf_map;

static GENRES_BY_ID: phf::Map<u64, IgdbGenre> = phf_map! {
    2u64 => IgdbGenre::PointAndClick,
    4u64 => IgdbGenre::Fighting,
    5u64 => IgdbGenre::Shooter,
    7u64 => IgdbGenre::Music,
    8u64 => IgdbGenre::Platformer,
    9u64 => IgdbGenre::Puzzle,
    10u64 => IgdbGenre::Racing,
    11u64 => IgdbGenre::RealTimeStrategy,
    12u64 => IgdbGenre::RPG,
    13u64 => IgdbGenre::Simulator,
    14u64 => IgdbGenre::Sports,
    15u64 => IgdbGenre::Strategy,
    16u64 => IgdbGenre::TurnBasedStrategy,
    24u64 => IgdbGenre::Tactical,
    25u64 => IgdbGenre::HackAndSlash,
    26u64 => IgdbGenre::Quiz,
    30u64 => IgdbGenre::Pinball,
    31u64 => IgdbGenre::Adventure,
    32u64 => IgdbGenre::Indie,
    33u64 => IgdbGenre::Arcade,
    34u64 => IgdbGenre::VisualNovel,
    35u64 => IgdbGenre::CardAndBoard,
    36u64 => IgdbGenre::MOBA,
};
