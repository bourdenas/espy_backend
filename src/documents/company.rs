use serde::{Deserialize, Serialize};

use super::{GameDigest, Image};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Company {
    pub id: u64,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub slug: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<Image>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub developed: Vec<GameDigest>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub published: Vec<GameDigest>,
}
