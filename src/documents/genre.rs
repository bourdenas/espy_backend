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

#[derive(Serialize, Deserialize, Default, Debug)]
pub enum EspyGenre {
    #[default]
    Unknown,

    // Adventure
    PointAndClick,
    Action,
    IsometricAction,
    NarrativeAdventure,
    SurvivalAdventure,
    PuzzleAdventure,
    WalkingSimulator,

    // Arcade
    Fighting,
    BeatEmUp,
    Pinball,
    CardAndBoard,
    Deckbuilder,

    // Casual
    LifeSim,
    FarmingSim,
    FishingSim,
    SailingSim,
    DatingSim,
    Puzzle,
    EndlessRunner,
    Rhythm,
    PartyGame,
    VisualNovel,
    Exploration,

    // Platformer
    SideScroller,
    Metroidvania,
    Platformer3d,
    ShooterPlatformer,
    PrecisionPlatformer,
    PuzzlePlatformer,

    // RPG
    CRPG,
    ARPG,
    ActionRpg,
    JRPG,
    FirstPersonRpg,
    TurnBasedRpg,
    RTwPRPG,
    DungeonCrawler,
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
    FlightSimulator,
    CombatSimulator,
    NavalSimulator,
    DrivingSimulator,
    Survival,

    // Strategy
    TurnBasedStrategy,
    RealTimeStrategy,
    TurnBasedTactics,
    RealTimeTactics,
    GradStrategy,
    FourX,
    TowerDefense,
    MOBA,
}

impl From<&str> for EspyGenre {
    fn from(description: &str) -> Self {
        match description {
            "Point & Click" => EspyGenre::PointAndClick,
            "Action" => EspyGenre::Action,
            "Isometric Action" => EspyGenre::IsometricAction,
            "Narrative Adventure" => EspyGenre::NarrativeAdventure,
            "Survival Adventure" => EspyGenre::SurvivalAdventure,
            "Puzzle Adventure" => EspyGenre::PuzzleAdventure,
            "Walking Simulator" => EspyGenre::WalkingSimulator,
            "Fighting" => EspyGenre::Fighting,
            "Beat\"em Up" => EspyGenre::BeatEmUp,
            "Pinball" => EspyGenre::Pinball,
            "Card & Board Game" => EspyGenre::CardAndBoard,
            "Deckbuilder" => EspyGenre::Deckbuilder,
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
            "Side Scroller" => EspyGenre::SideScroller,
            "Metroidvania" => EspyGenre::Metroidvania,
            "3D Platformer" => EspyGenre::Platformer3d,
            "Shooter Platformer" => EspyGenre::ShooterPlatformer,
            "Precision Platformer" => EspyGenre::PrecisionPlatformer,
            "Puzzle Platformer" => EspyGenre::PuzzlePlatformer,
            "CRPG" => EspyGenre::CRPG,
            "ARPG" => EspyGenre::ARPG,
            "Action RPG" => EspyGenre::ActionRpg,
            "JRPG" => EspyGenre::JRPG,
            "First Person RPG" => EspyGenre::FirstPersonRpg,
            "Turn Based RPG" => EspyGenre::TurnBasedRpg,
            "RTwP RPG" => EspyGenre::RTwPRPG,
            "Dungeon Crawler" => EspyGenre::DungeonCrawler,
            "MMORPG" => EspyGenre::MMORPG,
            "First Person Shooter" => EspyGenre::FirstPersonShooter,
            "Top-Down Shooter" => EspyGenre::TopDownShooter,
            "3rd Person Shooter" => EspyGenre::ThirdPersonShooter,
            "Space Shooter" => EspyGenre::SpaceShooter,
            "Shmup" => EspyGenre::Shmup,
            "Battle Royale" => EspyGenre::BattleRoyale,
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
