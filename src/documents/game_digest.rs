use std::collections::HashSet;

use itertools::Itertools;
use phf::phf_map;
use serde::{Deserialize, Serialize};

use super::{EspyGenre, GameCategory, GameEntry, GameStatus, IgdbGenre, Scores};

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
    pub espy_genres: Vec<EspyGenre>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub igdb_genres: Vec<IgdbGenre>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

impl From<GameEntry> for GameDigest {
    fn from(game_entry: GameEntry) -> Self {
        let keywords = extract_keywords(&game_entry);

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
                .map(|company| company.slug)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect(),

            publishers: game_entry
                .publishers
                .into_iter()
                .map(|company| company.slug)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect(),

            espy_genres: game_entry.espy_genres,
            igdb_genres: game_entry.igdb_genres,
            keywords,
        }
    }
}

fn extract_keywords(game_entry: &GameEntry) -> Vec<String> {
    let mut keywords = HashSet::<String>::default();

    let mut original_kws = vec![&game_entry.keywords];
    if let Some(steam_data) = &game_entry.steam_data {
        original_kws.push(&steam_data.user_tags);
    }
    if let Some(gog_data) = &game_entry.gog_data {
        original_kws.push(&gog_data.tags);
    }

    let original_kws = original_kws.into_iter().flatten().collect_vec();
    for kw in original_kws {
        let kw = kw.to_lowercase().replace("-", "").replace(" ", "");
        for kw_set in KW_SETS {
            if let Some(kw) = kw_set.get(&kw) {
                keywords.insert(kw.to_string());
                break;
            }
        }
    }

    keywords.into_iter().collect()
}

static KW_SETS: [&'static phf::Map<&'static str, &'static str>; 7] = [
    &SETTING_KWS,
    &HISTORICAL_SETTING_KWS,
    &GAMEPLAY_KWS,
    &VISUAL_STYLE_KWS,
    &MATURE_KWS,
    &MULTIPLAYER_KWS,
    &TRIGGER_KWS,
];

static SETTING_KWS: phf::Map<&'static str, &'static str> = phf_map! {
    "aliens" => "aliens",
    "alien" => "aliens",
    "vampires" => "vampires",
    "vampire" => "vampires",
    "zombies" => "zombies",
    "zombie" => "zombies",
    "mechs" => "mechs",
    "mech" => "mechs",

    "scifi" => "sci-fi",
    "cyberpunk" => "cyberpunk",
    "steampunk" => "steampunk",
    "darkfantasy" => "dark fantasy",
    "postapocalyptic" => "post-apocalyptic",
    "dystopian" => "dystopian",
    "lovecraftian" => "lovecraftian",
    "heavy metal" => "heavy metal",

    "space" => "space",
    "spacebattle" => "space",
    "spacecombat" => "space",
    "spacesim" => "space",
    "spacesimulation" => "space",

    "noir" => "noir",
    "filmnoir" => "noir",
    "timetravel" => "time travel",
};

static HISTORICAL_SETTING_KWS: phf::Map<&'static str, &'static str> = phf_map! {
    "ancientgreece" => "ancient world",
    "romanempire" => "ancient world",
    "rome" => "ancient world",

    "mythology" => "mythology",
    "greekmythology" => "mythology",

    "coldwar" => "cold war",
    "worldwari" => "WW1",
    "worldwariww1" => "WW1",
    "worldwarii" => "WW2",
    "worldwariiww2" => "WW2",
    "modernwarfare" => "modern warefare",
    "modernmilitary" => "modern warefare",

    "historical" => "historical",
    "alternatehistory" => "alternate history",
    "alternativehistory" => "alternate history",
};

static GAMEPLAY_KWS: phf::Map<&'static str, &'static str> = phf_map! {
    "roguelike" => "roguelike",
    "roguelite" => "roguelite",
    "turnbased" => "turn-based",
    "turnbasedcombat" => "turn-based",
    "tacticalturnbasedcombat" => "turn-based",
    "turnbasedrpg" => "turn-based",
    "rtwp" => "RTwP",
    "realtimewithpause" => "RTwP",
    "pausablerealtimecombal" => "RTwP",
    "dungeoncrawler" => "dungeon crawler",
    "boomershooter" => "boomer shooter",
    "lootershooter" => "looter shooter",
    "bullethell" => "bullet hell",
    "bullettime" => "bullet hell",
    "metroidvania" => "metroidvania",
    "precisionplatformer" => "precision platformer",
    "twinstickshooter" => "twin stick shooter",
    "soulslike" => "souls-like",
    "indie" => "indie",
};

static VISUAL_STYLE_KWS: phf::Map<&'static str, &'static str> = phf_map! {
    "anime" => "anime",
    "cartoon" => "cartoon",
    "cartoongraphics" => "cartoon",
    "cartoony" => "cartoon",
    "handdrawn" => "hand-drawn",
    "fmv" => "FMV",
    "fullmotionvideo" => "FMV",
    "pixelart" => "pixel art",
    "pixelgraphics" => "pixel art",
    "voxel" => "voxel",
};

static MATURE_KWS: phf::Map<&'static str, &'static str> = phf_map! {
    "adult" => "mature",
    "mature" => "mature",
    "horror" => "horror",
    "psychologicalhorror" => "psychological horror",
    "psychologicalthriller" => "psychological horror",
    "nsfw" => "NSFW",
    "nudity" => "nudity",
    "sexualcontent" => "sexual content",
    "familyfriendly" => "family friendly",
};

static MULTIPLAYER_KWS: phf::Map<&'static str, &'static str> = phf_map! {
    "coop" => "co-op",
    "coopcampaign" => "co-op",
    "localcoop" => "co-op",
    "onlinecoop" => "co-op",
    "pvp" => "PvP",
    "playervsplayer" => "PvP",
    "playervplayer" => "PvP",
};

static TRIGGER_KWS: phf::Map<&'static str, &'static str> = phf_map! {
    "freetoplay" => "free-to-play",
    "microtransaction" => "microtransaction",
    "paytoplay" => "pay-to-play",
};
