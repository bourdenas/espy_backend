use serde::{Deserialize, Serialize};

use super::GameDigest;

/// Document for 'espy/timeline' that contains info for building recent and
/// upcoming releases.
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

/// Document for 'espy/{year}' that contains info for building the annual
/// review.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct AnnualReview {
    #[serde(default)]
    pub last_updated: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub releases: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub indies: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub early_access: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub debug: Vec<GameDigest>,
}
