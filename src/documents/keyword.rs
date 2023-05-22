use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Keyword {
    pub id: u64,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub slug: String,

    #[serde(default)]
    pub url: String,
}
