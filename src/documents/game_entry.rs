use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use valuable::Valuable;

use super::{EspyGenre, GameDigest, GogData, IgdbGame, Scores, SteamData};

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
    pub igdb_genres: Vec<IgdbGenreType>,

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
    pub steam_appid: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam_data: Option<SteamData>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gog_data: Option<GogData>,
}

impl GameEntry {
    pub fn is_indie(&self) -> bool {
        self.release_year() > 2007
            && self
                .igdb_genres
                .iter()
                .any(|genre| matches!(genre, IgdbGenreType::Indie))
    }

    pub fn is_early_access(&self) -> bool {
        self.release_year() > 2018
            && matches!(self.status, GameStatus::EarlyAccess)
            && self.scores.metacritic.is_none()
    }

    pub fn is_released(&self) -> bool {
        self.has_release_date() && self.release_date < Utc::now().naive_utc().and_utc().timestamp()
    }
    pub fn has_release_date(&self) -> bool {
        self.release_date > 0
    }

    pub fn release_year(&self) -> i32 {
        DateTime::from_timestamp(self.release_date, 0)
            .unwrap()
            .year()
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
        self.steam_appid = Some(steam_data.steam_appid);
        self.steam_data = Some(steam_data);
    }

    pub fn add_gog_data(&mut self, gog_data: GogData) {
        self.scores.add_gog(&gog_data);
        self.gog_data = Some(gog_data);
    }

    pub fn update_igdb(&mut self, igdb_game: IgdbGame) {
        self.name = igdb_game.name.clone();
        self.category = Self::extract_category(&igdb_game);
        self.status = GameStatus::from(igdb_game.status);
        self.scores.add_igdb(&igdb_game);

        self.igdb_game = igdb_game;
        self.resolve_genres();
    }

    /// Determines the GameCategory of the GameEntry.
    ///
    /// The function tries to account for a common mistake in IGDB data that
    /// entries that have a version parent are not annotated correctly as
    /// Version.
    fn extract_category(igdb_game: &IgdbGame) -> GameCategory {
        match igdb_game.version_parent {
            Some(_) => GameCategory::Version,
            None => GameCategory::from(igdb_game.category),
        }
    }

    /// Produces IGDB genre descriptions based on igdb_game genre ids.
    ///
    /// There are only a handful IGDB genres so the mapping is hard-coded.
    fn resolve_genres(&mut self) {
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
}

impl From<IgdbGame> for GameEntry {
    fn from(igdb_game: IgdbGame) -> Self {
        let mut game_entry = GameEntry {
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
        };
        game_entry.resolve_genres();
        game_entry
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Copy, Debug)]
pub enum GamePlatform {
    PC,
    DOS,
    C64,
    Amiga,
    AtariST,
    Linux,
    Mac,
    Android,
    IOS,
    PS1,
    PS2,
    PS3,
    PS4,
    PS5,
    Xbox,
    Xbox360,
    XboxOne,
    XboxXS,
    Switch,
    Other,
}

impl From<u64> for GamePlatform {
    fn from(platform_id: u64) -> Self {
        match platform_id {
            6 => GamePlatform::PC,
            13 => GamePlatform::DOS,
            15 => GamePlatform::C64,
            16 => GamePlatform::Amiga,
            63 => GamePlatform::AtariST,
            3 => GamePlatform::Linux,
            14 => GamePlatform::Mac,
            34 => GamePlatform::Android,
            39 => GamePlatform::IOS,
            7 => GamePlatform::PS1,
            8 => GamePlatform::PS2,
            9 => GamePlatform::PS3,
            48 => GamePlatform::PS4,
            167 => GamePlatform::PS5,
            11 => GamePlatform::Xbox,
            12 => GamePlatform::Xbox360,
            49 => GamePlatform::XboxOne,
            169 => GamePlatform::XboxXS,
            130 => GamePlatform::Switch,
            _ => GamePlatform::Other,
        }
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Copy, Debug)]
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
    Updated,
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

    pub fn is_remaster(&self) -> bool {
        matches!(self, GameCategory::Remaster | GameCategory::Remake)
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
            9 => GameCategory::Remaster,
            10 | 14 => GameCategory::Updated,
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

#[derive(Serialize, Deserialize, Valuable, Clone, Copy, Debug)]
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
pub enum IgdbGenreType {
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

static GENRES_BY_ID: phf::Map<u64, IgdbGenreType> = phf_map! {
    2u64 => IgdbGenreType::PointAndClick,
    4u64 => IgdbGenreType::Fighting,
    5u64 => IgdbGenreType::Shooter,
    7u64 => IgdbGenreType::Music,
    8u64 => IgdbGenreType::Platformer,
    9u64 => IgdbGenreType::Puzzle,
    10u64 => IgdbGenreType::Racing,
    11u64 => IgdbGenreType::RealTimeStrategy,
    12u64 => IgdbGenreType::RPG,
    13u64 => IgdbGenreType::Simulator,
    14u64 => IgdbGenreType::Sports,
    15u64 => IgdbGenreType::Strategy,
    16u64 => IgdbGenreType::TurnBasedStrategy,
    24u64 => IgdbGenreType::Tactical,
    25u64 => IgdbGenreType::HackAndSlash,
    26u64 => IgdbGenreType::Quiz,
    30u64 => IgdbGenreType::Pinball,
    31u64 => IgdbGenreType::Adventure,
    32u64 => IgdbGenreType::Indie,
    33u64 => IgdbGenreType::Arcade,
    34u64 => IgdbGenreType::VisualNovel,
    35u64 => IgdbGenreType::CardAndBoard,
    36u64 => IgdbGenreType::MOBA,
};
