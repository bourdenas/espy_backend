use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::debug;
use valuable::Valuable;

use crate::{
    documents::{ExternalGame, IgdbGame, Keyword},
    logging::LogRequest,
};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum LogWebhooksRequest {
    AddGame(Document),
    UpdateGame(Document),
    ExternalGame(ExternalGameLog),
    Keyword(Document),
}

impl LogWebhooksRequest {
    pub fn add_game(igdb_game: &IgdbGame) {
        debug!(
            request = LogRequest::Webhooks(LogWebhooksRequest::AddGame(Document {
                id: igdb_game.id,
                name: igdb_game.name.clone(),
            }))
            .encode()
        )
    }

    pub fn update_game(igdb_game: &IgdbGame) {
        debug!(
            request = LogRequest::Webhooks(LogWebhooksRequest::UpdateGame(Document {
                id: igdb_game.id,
                name: igdb_game.name.clone(),
            }))
            .encode()
        )
    }

    pub fn external_game(external: &ExternalGame) {
        debug!(
            request = LogRequest::Webhooks(LogWebhooksRequest::ExternalGame(ExternalGameLog {
                id: external.igdb_id,
                name: external.name.clone(),
                store: external.store_name.to_string(),
            }))
            .encode()
        )
    }

    pub fn keyword(keyword: &Keyword) {
        debug!(
            request = LogRequest::Webhooks(LogWebhooksRequest::Keyword(Document {
                id: keyword.id,
                name: keyword.name.clone(),
            }))
            .encode()
        )
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
struct Document {
    id: u64,
    name: String,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
struct ExternalGameLog {
    id: u64,
    name: String,
    store: String,
}
