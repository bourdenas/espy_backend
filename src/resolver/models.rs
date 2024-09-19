use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SearchRequest {
    pub title: String,

    #[serde(default)]
    pub base_game_only: bool,
}
