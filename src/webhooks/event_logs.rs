use tracing::{error, info};

use crate::{
    documents::{ExternalGame, Keyword},
    Status,
};

pub struct ExternalGameEvent;

impl ExternalGameEvent {
    pub fn new() -> Self {
        ExternalGameEvent {}
    }

    pub fn log(self, external_game: ExternalGame) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = EXTERNAL_GAME_HANDLER,
            extenal_game.store = external_game.store_name,
            extenal_game.store_id = external_game.store_id,
            extenal_game.igdb_id = external_game.igdb_id,
            "update '{}' mapping on {}",
            external_game.name,
            external_game.store_name,
        )
    }

    pub fn log_unsupported(self, external_game: ExternalGame) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = EXTERNAL_GAME_HANDLER,
            extenal_game.store = "unsupported",
            extenal_game.store_id = external_game.store_id,
            extenal_game.igdb_id = external_game.igdb_id,
            "ignored '{}' mapping on unsupported store",
            external_game.store_name,
        )
    }

    pub fn log_error(self, external_game: ExternalGame, status: Status) {
        error!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = EXTERNAL_GAME_HANDLER,
            labels.status = status.to_string(),
            extenal_game.store = external_game.store_name,
            extenal_game.store_id = external_game.store_id,
            extenal_game.igdb_id = external_game.igdb_id,
            "failed to update '{}' game on {}",
            external_game.name,
            external_game.store_name,
        )
    }
}

pub struct KeywordsEvent {
    keyword: Keyword,
}

impl KeywordsEvent {
    pub fn new(keyword: Keyword) -> Self {
        KeywordsEvent { keyword }
    }

    pub fn log(self) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = KEYWORDS_HANDLER,
            keyword.id = self.keyword.id,
            keyword.slug = self.keyword.slug,
            "keyword updated"
        )
    }

    pub fn log_error(self, status: Status) {
        error!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = KEYWORDS_HANDLER,
            labels.status = status.to_string(),
            keyword.id = self.keyword.id,
            keyword.slug = self.keyword.slug,
            "failed to update external keyword"
        )
    }
}

const WEBHOOK_LOGS: &str = "webhook_logs";
const EXTERNAL_GAME_HANDLER: &str = "post_external_game";
const KEYWORDS_HANDLER: &str = "post_keywords";
