use tracing::{error, info};

use crate::{
    documents::{ExternalGame, IgdbGameDiff, Keyword},
    Status,
};

use super::{filtering::RejectionReason, prefiltering::PrefilterRejectionReason};

pub struct UpdateGameEvent {
    game_id: u64,
    game_name: String,
    handler_name: String,
}

impl UpdateGameEvent {
    pub fn new(game_id: u64, game_name: String, handler_name: String) -> Self {
        UpdateGameEvent {
            game_id,
            game_name,
            handler_name,
        }
    }

    pub fn log_added(self) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = self.handler_name,
            update_game.id = self.game_id,
            update_game.name = self.game_name,
            update_game.added = true,
            "added game {}",
            self.game_id
        )
    }

    pub fn log_updated(self, diff: IgdbGameDiff) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = self.handler_name,
            update_game.id = self.game_id,
            update_game.name = self.game_name,
            update_game.diff = diff.to_string(),
            "updated game {}",
            self.game_id
        )
    }

    pub fn log_no_update(self) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = self.handler_name,
            update_game.id = self.game_id,
            update_game.name = self.game_name,
            update_game.diff = "none".to_owned(),
            "nothing to update for {}",
            self.game_id
        )
    }

    pub fn log_reject(self, rejection: RejectionReason) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = self.handler_name,
            labels.rejection = rejection.to_string(),
            update_game.id = self.game_id,
            update_game.name = self.game_name,
            "rejected game {}",
            self.game_id
        )
    }

    pub fn log_prefilter_reject(self, rejection: PrefilterRejectionReason) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = self.handler_name,
            labels.rejection = rejection.to_string(),
            update_game.id = self.game_id,
            update_game.name = self.game_name,
            "prefilter game {}",
            self.game_id
        )
    }

    pub fn log_error(self, status: Status) {
        error!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = self.handler_name,
            labels.status = status.to_string(),
            update_game.id = self.game_id,
            update_game.name = self.game_name,
            "failed to update game {}",
            self.game_id
        )
    }
}

pub struct ExternalGameEvent {
    external_game: ExternalGame,
}

impl ExternalGameEvent {
    pub fn new(external_game: ExternalGame) -> Self {
        ExternalGameEvent { external_game }
    }

    pub fn log(self) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = EXTERNAL_GAME_HANDLER,
            extenal_game.store = self.external_game.store_name,
            extenal_game.store_id = self.external_game.store_id,
            extenal_game.igdb_id = self.external_game.igdb_id,
            "external game updated"
        )
    }

    pub fn log_error(self, status: Status) {
        error!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = EXTERNAL_GAME_HANDLER,
            labels.status = status.to_string(),
            extenal_game.store = self.external_game.store_name,
            extenal_game.store_id = self.external_game.store_id,
            extenal_game.igdb_id = self.external_game.igdb_id,
            "failed to update external game"
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
