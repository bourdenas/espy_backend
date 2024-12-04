use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{log_event, logging::LogEvent};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct MetacriticEvent {
    url: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl MetacriticEvent {
    pub fn scrape_game_page(url: String, error: Option<String>) {
        log_event!(LogEvent::Metacritic(MetacriticEvent { url, error }));
    }
}
