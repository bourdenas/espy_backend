use serde::{Deserialize, Serialize};

use super::GameDigest;

/// Document for 'espy/frontpage' that contains info for building the frontpage.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Frontpage {
    #[serde(default)]
    pub last_updated: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub upcoming: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub most_anticipated: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub critical_hits: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub popular: Vec<GameDigest>,
}
