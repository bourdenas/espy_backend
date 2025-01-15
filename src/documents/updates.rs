use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct DayUpdates {
    pub date: String,

    pub updates: Vec<Update>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Update {
    pub game_id: u64,

    pub date: u64,
    pub url: String,
    pub title: String,
    pub contents: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
}
