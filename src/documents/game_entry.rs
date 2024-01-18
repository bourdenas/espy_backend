use serde::{Deserialize, Serialize};

use crate::api::IgdbGame;

use super::{GameDigest, Scores, SteamData};

/// Document type under 'users/{user_id}/games' that represents an espy game
/// entry.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct GameEntry {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub category: GameCategory,

    #[serde(default)]
    pub status: GameStatus,

    #[serde(default)]
    pub last_updated: u64,

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
    pub espy_subgenres: Vec<EspySubGenre>,

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

impl GameEntry {
    pub fn resolve_genres(&mut self) {
        self.espy_genres = self
            .igdb_game
            .genres
            .iter()
            .filter_map(|igdb_genre_id| match GENRES_BY_ID.get(&igdb_genre_id) {
                Some(genre) => Some(*genre),
                None => None,
            })
            .collect();

        self.espy_subgenres = self
            .igdb_game
            .genres
            .iter()
            .filter_map(|igdb_genre_id| match SUBGENRES_BY_ID.get(&igdb_genre_id) {
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
        self.scores.update(&steam_data, self.release_date);
        self.steam_data = Some(steam_data);
    }
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

            release_date: match igdb_game.first_release_date {
                Some(timestamp) => timestamp,
                None => 0,
            },
            scores: Scores::from(&igdb_game),

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

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum EspyGenre {
    Adventure,
    Arcade,
    Online,
    Platformer,
    RPG,
    Shooter,
    Simulator,
    Strategy,
    Indie,
}

use phf::phf_map;

static GENRES_BY_ID: phf::Map<u64, EspyGenre> = phf_map! {
    2u64 => EspyGenre::Adventure,
    4u64 => EspyGenre::Arcade,
    5u64 => EspyGenre::Shooter,
    8u64 => EspyGenre::Platformer,
    10u64 => EspyGenre::Simulator,
    11u64 => EspyGenre::Strategy,
    12u64 => EspyGenre::RPG,
    13u64 => EspyGenre::Simulator,
    14u64 => EspyGenre::Simulator,
    15u64 => EspyGenre::Strategy,
    16u64 => EspyGenre::Strategy,
    24u64 => EspyGenre::Strategy,
    25u64 => EspyGenre::Arcade,
    30u64 => EspyGenre::Arcade,
    31u64 => EspyGenre::Adventure,
    32u64 => EspyGenre::Indie,
    33u64 => EspyGenre::Arcade,
    35u64 => EspyGenre::Arcade,
    36u64 => EspyGenre::Online,
};

static SUBGENRES_BY_ID: phf::Map<u64, EspySubGenre> = phf_map! {
    2u64 => EspySubGenre::PointAndClick,
    4u64 => EspySubGenre::Fighting,
    5u64 => EspySubGenre::FirstPersonShooter,
    10u64 => EspySubGenre::Racing,
    11u64 => EspySubGenre::RealTimeStrategy,
    14u64 => EspySubGenre::Sport,
    16u64 => EspySubGenre::TurnBasedStrategy,
    24u64 => EspySubGenre::TurnBasedTactics,
    25u64 => EspySubGenre::HackAndSlash,
    30u64 => EspySubGenre::Pinball,
    35u64 => EspySubGenre::CardBoardGame,
    36u64 => EspySubGenre::Moba,
};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum EspySubGenre {
    //  Adventure
    PointAndClick,
    Action,
    IsometricAction,
    IsometricAdventure,
    FirstPersonAdventure,
    NarrativeAdventure,
    PuzzleAdventure,

    // RPG
    CRpg,
    IsometricRpg,
    TurnBasedRpg,
    RTwPRpg,
    FirstPersonRpg,
    ActionRpg,
    HackAndSlash,
    JRpg,

    // Strategy
    TurnBasedStrategy,
    RealTimeStrategy,
    TurnBasedTactics,
    RealTimeTactics,
    IsometricTactics,
    GrandStrategy,
    XXXX,

    // Arcade
    Fighting,
    Pinball,
    BeatemUp,
    Puzzle,
    TowerDefense,
    EndlessRunner,
    CardBoardGame,

    // Online
    MMORPG,
    Moba,
    BattleRoyale,
    Coop,
    PvP,

    // Platformer
    SideScroller,
    Metroidvania,
    ThreeDPlatformer,
    ShooterPlatformer,
    PuzzlePlatformer,

    // Shooter
    FirstPersonShooter,
    TopDownShooter,
    ThirdPersonShooter,
    SpaceShooter,
    StealthShooter,
    FirstPersonMelee,

    // Simulator
    CityBuilder,
    GodGame,
    Racing,
    Sport,
    FlightSimulator,
    Management,
    Survival,
}
