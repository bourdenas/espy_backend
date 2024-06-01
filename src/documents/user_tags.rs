use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct UserAnnotations {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<Genre>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub user_tags: Vec<UserTag>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Genre {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub game_ids: Vec<u64>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct UserTag {
    pub name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub game_ids: Vec<u64>,
}

impl UserAnnotations {
    pub fn new() -> Self {
        UserAnnotations::default()
    }
}
