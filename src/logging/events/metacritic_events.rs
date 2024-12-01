use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{log_event, logging::LogEvent};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct MetacriticEvent {
    url: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

impl MetacriticEvent {
    pub fn scrape_game_page(url: String, errors: Vec<String>) {
        log_event!(LogEvent::Metacritic(MetacriticEvent { url, errors }));
    }
}
