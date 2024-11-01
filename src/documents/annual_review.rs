use serde::{Deserialize, Serialize};

use super::GameDigest;

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
    pub below_fold: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub debug: Vec<GameDigest>,
}
