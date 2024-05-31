use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Genre {
    pub id: u64,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub slug: String,

    #[serde(default)]
    pub url: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub enum EspyGenre {
    #[default]
    Unknown = 0,

    // Adventure
    PointAndClick = 1,
    Action = 2,
    IsometricAction = 3,
    NarrativeAdventure = 4,
    SurvivalAdventure = 5,
    PuzzleAdventure = 6,
    WalkingSimulator = 7,

    // Arcade
    Fighting = 8,
    BeatEmUp = 9,
    Pinball = 10,
    CardAndBoard = 11,
    Deckbuilder = 12,

    // Casual
    LifeSim = 13,
    FarmingSim = 14,
    FishingSim = 15,
    SailingSim = 16,
    DatingSim = 17,
    Puzzle = 18,
    EndlessRunner = 19,
    Rhythm = 20,
    PartyGame = 21,
    VisualNovel = 22,
    Exploration = 23,

    // Platformer
    SideScroller = 24,
    Metroidvania = 25,
    Platformer3d = 26,
    ShooterPlatformer = 27,
    PrecisionPlatformer = 28,
    PuzzlePlatformer = 29,

    // RPG
    CRPG = 30,
    ARPG = 31,
    ActionRpg = 32,
    JRPG = 33,
    FirstPersonRpg = 34,
    TurnBasedRpg = 35,
    RTwPRPG = 36,
    DungeonCrawler = 37,
    MMORPG = 38,

    // Shooter
    FirstPersonShooter = 39,
    TopDownShooter = 40,
    ThirdPersonShooter = 41,
    SpaceShooter = 42,
    Shmup = 43,
    BattleRoyale = 44,

    // Simulator
    CityBuilder = 45,
    Tycoon = 46,
    GodGame = 47,
    Racing = 48,
    Sports = 49,
    FlightSimulator = 50,
    CombatSimulator = 51,
    NavalSimulator = 52,
    DrivingSimulator = 53,
    Survival = 54,

    // Strategy
    TurnBasedStrategy = 55,
    RealTimeStrategy = 56,
    TurnBasedTactics = 57,
    RealTimeTactics = 58,
    GradStrategy = 59,
    FourX = 60,
    TowerDefense = 61,
    MOBA = 62,
}

impl EspyGenre {
    pub fn from_user_tag(description: &str) -> Self {
        match description {
            // Adventure
            "Point & Click" => EspyGenre::PointAndClick,
            "Action" => EspyGenre::Action,
            "Isometric Action" => EspyGenre::IsometricAction,
            "Narrative Adventure" => EspyGenre::NarrativeAdventure,
            "Survival Adventure" => EspyGenre::SurvivalAdventure,
            "Puzzle Adventure" => EspyGenre::PuzzleAdventure,
            "Walking Simulator" => EspyGenre::WalkingSimulator,

            // Arcade
            "Fighting" => EspyGenre::Fighting,
            "Beat\"em Up" => EspyGenre::BeatEmUp,
            "Pinball" => EspyGenre::Pinball,
            "Card & Board Game" => EspyGenre::CardAndBoard,
            "Deckbuilder" => EspyGenre::Deckbuilder,

            // Casual
            "Life Sim" => EspyGenre::LifeSim,
            "Farming Sim" => EspyGenre::FarmingSim,
            "Fishing Sim" => EspyGenre::FishingSim,
            "Sailing Sim" => EspyGenre::SailingSim,
            "Dating Sim" => EspyGenre::DatingSim,
            "Puzzle" => EspyGenre::Puzzle,
            "Endless Runner" => EspyGenre::EndlessRunner,
            "Rhythm" => EspyGenre::Rhythm,
            "Party Game" => EspyGenre::PartyGame,
            "Visual Novel" => EspyGenre::VisualNovel,
            "Exploration" => EspyGenre::Exploration,

            // Platformer
            "Side Scroller" => EspyGenre::SideScroller,
            "Metroidvania" => EspyGenre::Metroidvania,
            "3D Platformer" => EspyGenre::Platformer3d,
            "Shooter Platformer" => EspyGenre::ShooterPlatformer,
            "Precision Platformer" => EspyGenre::PrecisionPlatformer,
            "Puzzle Platformer" => EspyGenre::PuzzlePlatformer,

            // RPG
            "CRPG" => EspyGenre::CRPG,
            "ARPG" => EspyGenre::ARPG,
            "Action RPG" => EspyGenre::ActionRpg,
            "JRPG" => EspyGenre::JRPG,
            "First Person RPG" => EspyGenre::FirstPersonRpg,
            "Turn Based RPG" => EspyGenre::TurnBasedRpg,
            "RTwP RPG" => EspyGenre::RTwPRPG,
            "Dungeon Crawler" => EspyGenre::DungeonCrawler,
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
            "Flight Simulator" => EspyGenre::FlightSimulator,
            "Combat Simulator" => EspyGenre::CombatSimulator,
            "Naval Simulator" => EspyGenre::NavalSimulator,
            "Driving Simulator" => EspyGenre::DrivingSimulator,
            "Survival" => EspyGenre::Survival,

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
            // Adventure
            "PointAndClick" => EspyGenre::PointAndClick,
            "Action" => EspyGenre::Action,
            "IsometricAction" => EspyGenre::IsometricAction,
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
            "FishingSim" => EspyGenre::FishingSim,
            "SailingSim" => EspyGenre::SailingSim,
            "DatingSim" => EspyGenre::DatingSim,
            "Puzzle" => EspyGenre::Puzzle,
            "EndlessRunner" => EspyGenre::EndlessRunner,
            "Rhythm" => EspyGenre::Rhythm,
            "PartyGame" => EspyGenre::PartyGame,
            "VisualNovel" => EspyGenre::VisualNovel,
            "Exploration" => EspyGenre::Exploration,

            // Platformer
            "SideScroller" => EspyGenre::SideScroller,
            "Metroidvania" => EspyGenre::Metroidvania,
            "Platformer3d" => EspyGenre::Platformer3d,
            "ShooterPlatformer" => EspyGenre::ShooterPlatformer,
            "PrecisionPlatformer" => EspyGenre::PrecisionPlatformer,
            "PuzzlePlatformer" => EspyGenre::PuzzlePlatformer,

            // RPG
            "CRPG" => EspyGenre::CRPG,
            "ARPG" => EspyGenre::ARPG,
            "ActionRpg" => EspyGenre::ActionRpg,
            "JRPG" => EspyGenre::JRPG,
            "FirstPersonRpg" => EspyGenre::FirstPersonRpg,
            "TurnBasedRpg" => EspyGenre::TurnBasedRpg,
            "RTwPRPG" => EspyGenre::RTwPRPG,
            "DungeonCrawler" => EspyGenre::DungeonCrawler,
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
            "FlightSimulator" => EspyGenre::FlightSimulator,
            "CombatSimulator" => EspyGenre::CombatSimulator,
            "NavalSimulator" => EspyGenre::NavalSimulator,
            "DrivingSimulator" => EspyGenre::DrivingSimulator,
            "Survival" => EspyGenre::Survival,

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
