use serde::{Deserialize, Serialize};

use super::GameDigest;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Company {
    pub id: u64,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub slug: String,

    #[serde(default)]
    pub logo: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub developed: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub published: Vec<GameDigest>,
}
