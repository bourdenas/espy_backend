use std::fmt::{Debug, Display};

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{
    documents::{ExternalGame, IgdbGame, Keyword},
    log_request,
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
        log_request!(LogRequest::Webhooks(LogWebhooksRequest::AddGame(
            Document {
                id: igdb_game.id,
                name: igdb_game.name.clone(),
            }
        )))
    }

    pub fn update_game(igdb_game: &IgdbGame) {
        log_request!(LogRequest::Webhooks(LogWebhooksRequest::UpdateGame(
            Document {
                id: igdb_game.id,
                name: igdb_game.name.clone(),
            }
        )))
    }

    pub fn external_game(external: &ExternalGame) {
        log_request!(LogRequest::Webhooks(LogWebhooksRequest::ExternalGame(
            ExternalGameLog {
                id: external.igdb_id,
                name: external.name.clone(),
                store: external.store_name.to_string(),
            }
        )))
    }

    pub fn keyword(keyword: &Keyword) {
        log_request!(LogRequest::Webhooks(LogWebhooksRequest::Keyword(
            Document {
                id: keyword.id,
                name: keyword.name.clone(),
            }
        )))
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct Document {
    id: u64,
    name: String,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct ExternalGameLog {
    id: u64,
    name: String,
    store: String,
}

impl Display for LogWebhooksRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogWebhooksRequest::AddGame(game) => {
                write!(f, "add_game '{}' ({})", game.name, game.id)
            }
            LogWebhooksRequest::UpdateGame(game) => {
                write!(f, "update_game '{}' ({})", game.name, game.id)
            }
            LogWebhooksRequest::ExternalGame(external_game) => {
                write!(
                    f,
                    "external_game from {} '{}' ({})",
                    external_game.store, external_game.name, external_game.id
                )
            }
            LogWebhooksRequest::Keyword(kw) => write!(f, "keyword '{}' ({})", kw.name, kw.id),
        }
    }
}
