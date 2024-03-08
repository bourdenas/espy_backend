use tracing::{error, info};

use crate::{
    api::IgdbGameDiff,
    documents::{ExternalGame, Genre, Keyword},
    Status,
};

use super::filltering::RejectionReason;

pub struct AddGameEvent {
    id: u64,
    name: String,
}

impl AddGameEvent {
    pub fn new(id: u64, name: String) -> Self {
        AddGameEvent { id, name }
    }

    pub fn log(self) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = ADD_GAME_HANDLER,
            add_game.id = self.id,
            add_game.name = self.name,
            "added game {}",
            self.id
        )
    }

    pub fn log_reject(self, rejection: RejectionReason) {
        error!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = UPDATE_GAME_HANDLER,
            labels.rejection = rejection.to_string(),
            update_game.id = self.id,
            update_game.name = self.name,
            "rejected game {}",
            self.id
        )
    }
    pub fn log_error(self, status: Status) {
        error!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = ADD_GAME_HANDLER,
            labels.status = status.to_string(),
            add_game.id = self.id,
            add_game.name = self.name,
            "failed to add game {}",
            self.id
        )
    }
}

pub struct UpdateGameEvent {
    id: u64,
    name: String,
}

impl UpdateGameEvent {
    pub fn new(id: u64, name: String) -> Self {
        UpdateGameEvent { id, name }
    }

    pub fn log(self, diff: Option<IgdbGameDiff>) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = UPDATE_GAME_HANDLER,
            update_game.id = self.id,
            update_game.name = self.name,
            update_game.diff = match diff {
                Some(diff) => diff.to_string(),
                None => "null".to_owned(),
            },
            "updated game {}",
            self.id
        )
    }

    pub fn log_added(self) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = UPDATE_GAME_HANDLER,
            update_game.id = self.id,
            update_game.name = self.name,
            update_game.added = true,
            "discovered game {}",
            self.id
        )
    }

    pub fn log_reject(self, rejection: RejectionReason) {
        error!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = UPDATE_GAME_HANDLER,
            labels.rejection = rejection.to_string(),
            update_game.id = self.id,
            update_game.name = self.name,
            "rejected game {}",
            self.id
        )
    }

    pub fn log_error(self, status: Status) {
        error!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = UPDATE_GAME_HANDLER,
            labels.status = status.to_string(),
            update_game.id = self.id,
            update_game.name = self.name,
            "failed to update game {}",
            self.id
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

pub struct GenresEvent {
    genre: Genre,
}

impl GenresEvent {
    pub fn new(genre: Genre) -> Self {
        GenresEvent { genre }
    }

    pub fn log(self) {
        info!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = GENRES_HANDLER,
            genre.id = self.genre.id,
            genre.slug = self.genre.slug,
            "genre updated"
        )
    }

    pub fn log_error(self, status: Status) {
        error!(
            labels.log_type = WEBHOOK_LOGS,
            labels.handler = GENRES_HANDLER,
            labels.status = status.to_string(),
            genre.id = self.genre.id,
            genre.slug = self.genre.slug,
            "failed to update external genre"
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
const ADD_GAME_HANDLER: &str = "post_add_game";
const UPDATE_GAME_HANDLER: &str = "post_update_game";
const EXTERNAL_GAME_HANDLER: &str = "post_external_game";
const GENRES_HANDLER: &str = "post_genres";
const KEYWORDS_HANDLER: &str = "post_keywords";
