use std::time::SystemTime;

use tracing::{error, info};

use crate::{documents::GameEntry, Status};

use super::models;

pub struct ResolveEvent {
    request: models::Resolve,
    start: SystemTime,
}

impl ResolveEvent {
    pub fn new(request: models::Resolve) -> Self {
        Self {
            request,
            start: SystemTime::now(),
        }
    }

    pub fn log(self, game_entry: GameEntry) {
        info!(
            http_request.request_method = "POST",
            http_request.request_url = "/resolve",
            labels.log_type = QUERY_LOGS,
            labels.handler = RESOLVE_HANDLER,
            request.game_id = self.request.game_id,
            resolve.title = game_entry.name,
            resolve.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "resolve {} => '{}'",
            self.request.game_id,
            game_entry.name
        )
    }

    pub fn log_error(self, status: Status) {
        error!(
            http_request.request_method = "POST",
            http_request.request_url = "/resolve",
            labels.log_type = QUERY_LOGS,
            labels.handler = RESOLVE_HANDLER,
            labels.status = status.to_string(),
            request.game_id = self.request.game_id,
            resolve.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "resolve {} => none",
            self.request.game_id
        )
    }
}

pub struct UpdateEvent<'a> {
    request: &'a models::UpdateOp,
    start: SystemTime,
}

impl<'a> UpdateEvent<'a> {
    pub fn new(request: &'a models::UpdateOp) -> Self {
        Self {
            request,
            start: SystemTime::now(),
        }
    }

    pub fn log(self, user_id: &str) {
        info!(
            http_request.request_method = "POST",
            http_request.request_url = "/library/_/update",
            labels.log_type = QUERY_LOGS,
            labels.handler = UPDATE_HANDLER,
            request.game_id = self.request.game_id,
            update.user_id = user_id,
            update.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "update {}",
            self.request.game_id,
        )
    }

    pub fn log_error(self, user_id: &str, status: Status) {
        error!(
            http_request.request_method = "POST",
            http_request.request_url = "/library/_/update",
            labels.log_type = QUERY_LOGS,
            labels.handler = UPDATE_HANDLER,
            labels.status = status.to_string(),
            request.game_id = self.request.game_id,
            update.user_id = user_id,
            update.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "update {}",
            self.request.game_id,
        )
    }
}

pub struct MatchEvent {
    request: models::MatchOp,
    start: SystemTime,
}

impl MatchEvent {
    pub fn new(request: models::MatchOp) -> Self {
        Self {
            request,
            start: SystemTime::now(),
        }
    }

    pub fn log(self, user_id: &str) {
        info!(
            http_request.request_method = "POST",
            http_request.request_url = "/library/_/match",
            labels.log_type = QUERY_LOGS,
            labels.handler = MATCH_HANDLER,
            request.op = self.op(),
            request.store_entry.store = self.request.store_entry.storefront_name,
            request.store_entry.game_id = self.request.store_entry.id,
            request.store_entry.game_title = self.request.store_entry.title,
            request.delete = self.request.delete_unmatched,
            match_op.user_id = user_id,
            match_op.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "{} '{}'",
            self.op(),
            self.request.store_entry.title,
        )
    }

    pub fn log_error(self, user_id: &str, status: Status) {
        error!(
            http_request.request_method = "POST",
            http_request.request_url = "/library/_/match",
            labels.log_type = QUERY_LOGS,
            labels.handler = MATCH_HANDLER,
            labels.status = status.to_string(),
            request.op = self.op(),
            request.store_entry.store = self.request.store_entry.storefront_name,
            request.store_entry.game_id = self.request.store_entry.id,
            request.store_entry.game_title = self.request.store_entry.title,
            request.delete = self.request.delete_unmatched,
            match_op.user_id = user_id,
            match_op.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "{} '{}'",
            self.op(),
            self.request.store_entry.title,
        )
    }

    fn op(&self) -> &'static str {
        match (&self.request.game_id, &self.request.unmatch_entry) {
            (Some(_), None) => "match",
            (None, Some(_)) => "unmatch",
            (Some(_), Some(_)) => "rematch",
            (None, None) => "bad_request",
        }
    }
}

pub struct WishlistEvent {
    request: models::WishlistOp,
    start: SystemTime,
}

impl WishlistEvent {
    pub fn new(request: models::WishlistOp) -> Self {
        Self {
            request,
            start: SystemTime::now(),
        }
    }

    pub fn log(self, user_id: &str) {
        info!(
            http_request.request_method = "POST",
            http_request.request_url = "/library/_/wishlist",
            labels.log_type = QUERY_LOGS,
            labels.handler = WISHLIST_HANDLER,
            request.op = self.op(),
            request.game_id = self.game_id(),
            wishlist.user_id = user_id,
            wishlist.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "{} '{}'",
            self.op(),
            self.game_id(),
        )
    }

    pub fn log_error(self, user_id: &str, status: Status) {
        error!(
            http_request.request_method = "POST",
            http_request.request_url = "/library/_/wishlist",
            labels.log_type = QUERY_LOGS,
            labels.handler = WISHLIST_HANDLER,
            labels.status = status.to_string(),
            request.op = self.op(),
            request.game_id = self.game_id(),
            wishlist.user_id = user_id,
            wishlist.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "{} '{}'",
            self.op(),
            self.game_id(),
        )
    }

    fn op(&self) -> &'static str {
        match (&self.request.add_game, &self.request.remove_game) {
            (Some(_), _) => "add_to_wishlist",
            (_, Some(_)) => "remove_from_wishlist",
            _ => "bad_request",
        }
    }

    fn game_id(&self) -> u64 {
        match (&self.request.add_game, &self.request.remove_game) {
            (Some(library_entry), _) => library_entry.id,
            (_, Some(id)) => *id,
            _ => 0,
        }
    }
}

pub struct UnlinkEvent<'a> {
    request: &'a models::Unlink,
    start: SystemTime,
}

impl<'a> UnlinkEvent<'a> {
    pub fn new(request: &'a models::Unlink) -> Self {
        Self {
            request,
            start: SystemTime::now(),
        }
    }

    pub fn log(self, user_id: &str) {
        info!(
            http_request.request_method = "POST",
            http_request.request_url = "/library/_/unlink",
            labels.log_type = QUERY_LOGS,
            labels.handler = UNLINK_HANDLER,
            request.storefront = self.request.storefront_id,
            unlink.user_id = user_id,
            unlink.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "unlink {}",
            self.request.storefront_id
        )
    }

    pub fn log_error(self, user_id: &str, status: Status) {
        error!(
            http_request.request_method = "POST",
            http_request.request_url = "/library/_/unlink",
            labels.log_type = QUERY_LOGS,
            labels.handler = UNLINK_HANDLER,
            labels.status = status.to_string(),
            request.storefront = self.request.storefront_id,
            unlink.user_id = user_id,
            unlink.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "unlink {}",
            self.request.storefront_id
        )
    }
}

pub struct SyncEvent {
    start: SystemTime,
}

impl SyncEvent {
    pub fn new() -> Self {
        Self {
            start: SystemTime::now(),
        }
    }

    pub fn log(self, user_id: &str) {
        info!(
            http_request.request_method = "POST",
            http_request.request_url = "/library/_/sync",
            labels.log_type = QUERY_LOGS,
            labels.handler = SYNC_HANDLER,
            sync.user_id = user_id,
            sync.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "sync"
        )
    }

    pub fn log_error(self, user_id: &str, status: Status) {
        error!(
            http_request.request_method = "POST",
            http_request.request_url = "/library/_/sync",
            labels.log_type = QUERY_LOGS,
            labels.handler = SYNC_HANDLER,
            labels.status = status.to_string(),
            sync.user_id = user_id,
            sync.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "sync"
        )
    }
}

const QUERY_LOGS: &str = "query_logs";
const RESOLVE_HANDLER: &str = "resolve";
const UPDATE_HANDLER: &str = "update";
const MATCH_HANDLER: &str = "match";
const WISHLIST_HANDLER: &str = "wishlist";
const UNLINK_HANDLER: &str = "unlink";
const SYNC_HANDLER: &str = "sync";
