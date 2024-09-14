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
    Unknown,

    // Action
    Action,
    IsometricAction,
    ActionRpg,

    // Adventure
    PointAndClick,
    NarrativeAdventure,
    SurvivalAdventure,
    PuzzleAdventure,
    WalkingSimulator,

    // Arcade
    Fighting,
    BeatEmUp,
    Pinball,
    CardAndBoard,

    // Casual
    LifeSim,
    FarmingSim,
    DatingSim,
    Puzzle,
    VisualNovel,
    Exploration,
    EndlessRunner,
    Rhythm,
    PartyGame,

    // Platformer
    SideScroller,
    Platformer3d,
    ShooterPlatformer,
    PuzzlePlatformer,

    // RPG
    CRPG,
    ARPG,
    FirstPersonRpg,
    JRPG,
    MMORPG,

    // Shooter
    FirstPersonShooter,
    TopDownShooter,
    ThirdPersonShooter,
    SpaceShooter,
    Shmup,
    BattleRoyale,

    // Simulator
    CityBuilder,
    Tycoon,
    GodGame,
    Racing,
    Sports,
    Survival,
    FlightSimulator,
    CombatSimulator,
    DrivingSimulator,
    NavalSimulator,

    // Strategy
    TurnBasedStrategy,
    RealTimeStrategy,
    TurnBasedTactics,
    RealTimeTactics,
    GradStrategy,
    FourX,
    TowerDefense,
    MOBA,

    // Obsolete
    TurnBasedRpg,
    RTwPRPG,
    DungeonCrawler,
    Metroidvania,
    PrecisionPlatformer,
    FishingSim,
    SailingSim,
    Deckbuilder,
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
