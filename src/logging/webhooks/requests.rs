use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::debug;
use valuable::Valuable;

use crate::{documents::IgdbGame, logging::LogRequest};

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
pub enum LogWebhooksRequest {
    #[default]
    None,

    AddGame(Game),
    UpdateGame(Game),
    // ExternalGame(IgdbExternalGame, Status),
    // Keyword(Keyword, Status),
}

impl LogWebhooksRequest {
    pub fn add_game(igdb_game: &IgdbGame) {
        debug!(
            request = LogRequest::Webhooks(LogWebhooksRequest::AddGame(Game {
                id: igdb_game.id,
                name: igdb_game.name.clone(),
            }))
            .encode()
        );
    }

    pub fn update_game(igdb_game: &IgdbGame) {
        debug!(
            request = LogRequest::Webhooks(LogWebhooksRequest::UpdateGame(Game {
                id: igdb_game.id,
                name: igdb_game.name.clone(),
            }))
            .encode()
        );
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
struct Game {
    id: u64,
    name: String,
}
