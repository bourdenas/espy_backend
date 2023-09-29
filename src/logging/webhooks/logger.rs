use tracing::{error, info};

use crate::{api::IgdbGameDiff, Status};

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
            "added {}",
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
            "failed to add {}",
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
            "updated {}",
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
            "updated {}",
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
            "failed to update {}",
            self.id
        )
    }
}

const WEBHOOK_LOGS: &str = "webhook_logs";
const ADD_GAME_HANDLER: &str = "post_add_game";
const UPDATE_GAME_HANDLER: &str = "post_update_game";
