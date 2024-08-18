use serde::{Deserialize, Serialize};

/// Document type under 'genres' collection for quick lookup for
/// game_id -> EspyGenres.
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Genre {
    pub game_id: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub espy_genres: Vec<EspyGenre>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub enum EspyGenre {
    #[default]
    Unknown = 0,

    // Action
    Action = 1,
    IsometricAction = 2,
    ActionRpg = 3,

    // Adventure
    PointAndClick = 4,
    NarrativeAdventure = 5,
    SurvivalAdventure = 6,
    PuzzleAdventure = 7,
    WalkingSimulator = 8,

    // Arcade
    Fighting = 9,
    BeatEmUp = 10,
    Pinball = 11,
    CardAndBoard = 12,
    Deckbuilder = 13,

    // Casual
    LifeSim = 14,
    FarmingSim = 15,
    DatingSim = 16,
    Puzzle = 17,
    VisualNovel = 18,
    Exploration = 19,
    EndlessRunner = 20,
    Rhythm = 21,
    PartyGame = 22,

    // Platformer
    SideScroller = 23,
    Platformer3d = 24,
    ShooterPlatformer = 25,
    PuzzlePlatformer = 26,

    // RPG
    CRPG = 27,
    ARPG = 28,
    FirstPersonRpg = 29,
    JRPG = 30,
    MMORPG = 31,

    // Shooter
    FirstPersonShooter = 32,
    TopDownShooter = 33,
    ThirdPersonShooter = 34,
    SpaceShooter = 35,
    Shmup = 36,
    BattleRoyale = 37,

    // Simulator
    CityBuilder = 38,
    Tycoon = 39,
    GodGame = 40,
    Racing = 41,
    Sports = 42,
    Survival = 43,
    FlightSimulator = 44,
    CombatSimulator = 45,
    DrivingSimulator = 46,
    NavalSimulator = 47,

    // Strategy
    TurnBasedStrategy = 48,
    RealTimeStrategy = 49,
    TurnBasedTactics = 50,
    RealTimeTactics = 51,
    GradStrategy = 52,
    FourX = 53,
    TowerDefense = 54,
    MOBA = 55,

    // Obsolete
    TurnBasedRpg,
    RTwPRPG,
    DungeonCrawler,
    Metroidvania,
    PrecisionPlatformer,
    FishingSim,
    SailingSim,
}

impl EspyGenre {
    pub fn from_user_tag(description: &str) -> Self {
        match description {
            // Action
            "Action" => EspyGenre::Action,
            "Isometric Action" => EspyGenre::IsometricAction,
            "Action RPG" => EspyGenre::ActionRpg,

            // Adventure
            "Point & Click" => EspyGenre::PointAndClick,
            "Narrative Adventure" => EspyGenre::NarrativeAdventure,
            "Survival Adventure" => EspyGenre::SurvivalAdventure,
            "Puzzle Adventure" => EspyGenre::PuzzleAdventure,
            "Walking Simulator" => EspyGenre::WalkingSimulator,

            // Arcade
            "Fighting" => EspyGenre::Fighting,
            "Beat'em Up" => EspyGenre::BeatEmUp,
            "Pinball" => EspyGenre::Pinball,
            "Card & Board Game" => EspyGenre::CardAndBoard,
            "Deckbuilder" => EspyGenre::Deckbuilder,

            // Casual
            "Life Sim" => EspyGenre::LifeSim,
            "Farming Sim" => EspyGenre::FarmingSim,
            "Dating Sim" => EspyGenre::DatingSim,
            "Puzzle" => EspyGenre::Puzzle,
            "Visual Novel" => EspyGenre::VisualNovel,
            "Exploration" => EspyGenre::Exploration,
            "Endless Runner" => EspyGenre::EndlessRunner,
            "Rhythm" => EspyGenre::Rhythm,
            "Party Game" => EspyGenre::PartyGame,

            // Platformer
            "Side Scroller" => EspyGenre::SideScroller,
            "3D Platformer" => EspyGenre::Platformer3d,
            "Shooter Platformer" => EspyGenre::ShooterPlatformer,
            "Puzzle Platformer" => EspyGenre::PuzzlePlatformer,

            // RPG
            "CRPG" => EspyGenre::CRPG,
            "ARPG" => EspyGenre::ARPG,
            "First Person RPG" => EspyGenre::FirstPersonRpg,
            "JRPG" => EspyGenre::JRPG,
            "MMORPG" => EspyGenre::MMORPG,

            // Shooter
            "First Person Shooter" => EspyGenre::FirstPersonShooter,
            "Top-Down Shooter" => EspyGenre::TopDownShooter,
            "3rd Person Shooter" => EspyGenre::ThirdPersonShooter,
            "Space Shooter" => EspyGenre::SpaceShooter,
            "Shmup" => EspyGenre::Shmup,
            "Battle Royale" => EspyGenre::BattleRoyale,

            // Simulator
            "City Builder" => EspyGenre::CityBuilder,
            "Tycoon" => EspyGenre::Tycoon,
            "God Game" => EspyGenre::GodGame,
            "Racing" => EspyGenre::Racing,
            "Sports" => EspyGenre::Sports,
            "Survival" => EspyGenre::Survival,
            "Flight Simulator" => EspyGenre::FlightSimulator,
            "Combat Simulator" => EspyGenre::CombatSimulator,
            "Driving Simulator" => EspyGenre::DrivingSimulator,
            "Naval Simulator" => EspyGenre::NavalSimulator,

            // Strategy
            "Turn Based Strategy" => EspyGenre::TurnBasedStrategy,
            "Real-Time Strategy" => EspyGenre::RealTimeStrategy,
            "Turn Based Tactics" => EspyGenre::TurnBasedTactics,
            "Real-Time Tactics" => EspyGenre::RealTimeTactics,
            "Grand Strategy" => EspyGenre::GradStrategy,
            "4X" => EspyGenre::FourX,
            "Tower Defense" => EspyGenre::TowerDefense,
            "MOBA" => EspyGenre::MOBA,
            _ => EspyGenre::Unknown,
        }
    }
}

impl From<&str> for EspyGenre {
    fn from(s: &str) -> Self {
        match s {
            // Action
            "Action" => EspyGenre::Action,
            "IsometricAction" => EspyGenre::IsometricAction,
            "ActionRpg" => EspyGenre::ActionRpg,

            // Adventure
            "PointAndClick" => EspyGenre::PointAndClick,
            "NarrativeAdventure" => EspyGenre::NarrativeAdventure,
            "SurvivalAdventure" => EspyGenre::SurvivalAdventure,
            "PuzzleAdventure" => EspyGenre::PuzzleAdventure,
            "WalkingSimulator" => EspyGenre::WalkingSimulator,

            // Arcade
            "Fighting" => EspyGenre::Fighting,
            "BeatEmUp" => EspyGenre::BeatEmUp,
            "Pinball" => EspyGenre::Pinball,
            "CardAndBoard" => EspyGenre::CardAndBoard,
            "Deckbuilder" => EspyGenre::Deckbuilder,

            // Casual
            "LifeSim" => EspyGenre::LifeSim,
            "FarmingSim" => EspyGenre::FarmingSim,
            "DatingSim" => EspyGenre::DatingSim,
            "Puzzle" => EspyGenre::Puzzle,
            "VisualNovel" => EspyGenre::VisualNovel,
            "Exploration" => EspyGenre::Exploration,
            "EndlessRunner" => EspyGenre::EndlessRunner,
            "Rhythm" => EspyGenre::Rhythm,
            "PartyGame" => EspyGenre::PartyGame,

            // Platformer
            "SideScroller" => EspyGenre::SideScroller,
            "Platformer3d" => EspyGenre::Platformer3d,
            "ShooterPlatformer" => EspyGenre::ShooterPlatformer,
            "PuzzlePlatformer" => EspyGenre::PuzzlePlatformer,

            // RPG
            "CRPG" => EspyGenre::CRPG,
            "ARPG" => EspyGenre::ARPG,
            "FirstPersonRpg" => EspyGenre::FirstPersonRpg,
            "JRPG" => EspyGenre::JRPG,
            "MMORPG" => EspyGenre::MMORPG,

            // Shooter
            "FirstPersonShooter" => EspyGenre::FirstPersonShooter,
            "TopDownShooter" => EspyGenre::TopDownShooter,
            "ThirdPersonShooter" => EspyGenre::ThirdPersonShooter,
            "SpaceShooter" => EspyGenre::SpaceShooter,
            "Shmup" => EspyGenre::Shmup,
            "BattleRoyale" => EspyGenre::BattleRoyale,

            // Simulator
            "CityBuilder" => EspyGenre::CityBuilder,
            "Tycoon" => EspyGenre::Tycoon,
            "GodGame" => EspyGenre::GodGame,
            "Racing" => EspyGenre::Racing,
            "Sports" => EspyGenre::Sports,
            "Survival" => EspyGenre::Survival,
            "FlightSimulator" => EspyGenre::FlightSimulator,
            "CombatSimulator" => EspyGenre::CombatSimulator,
            "DrivingSimulator" => EspyGenre::DrivingSimulator,
            "NavalSimulator" => EspyGenre::NavalSimulator,

            // Strategy
            "TurnBasedStrategy" => EspyGenre::TurnBasedStrategy,
            "RealTimeStrategy" => EspyGenre::RealTimeStrategy,
            "TurnBasedTactics" => EspyGenre::TurnBasedTactics,
            "RealTimeTactics" => EspyGenre::RealTimeTactics,
            "GradStrategy" => EspyGenre::GradStrategy,
            "FourX" => EspyGenre::FourX,
            "TowerDefense" => EspyGenre::TowerDefense,
            "MOBA" => EspyGenre::MOBA,

            _ => EspyGenre::Unknown,
        }
    }
}
