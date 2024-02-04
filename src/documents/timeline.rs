use serde::{Deserialize, Serialize};

use super::GameDigest;

/// Document for 'espy/timeline' that contains info for building the frontpage.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct TimelineLegacy {
    #[serde(default)]
    pub last_updated: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub upcoming: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent: Vec<GameDigest>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Timeline {
    #[serde(default)]
    pub last_updated: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub releases: Vec<ReleaseEvent>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ReleaseEvent {
    pub label: String,

    pub year: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub games: Vec<GameDigest>,
}
