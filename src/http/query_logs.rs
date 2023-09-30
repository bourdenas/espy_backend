use std::time::SystemTime;

use tracing::{error, info};

use crate::{documents::GameEntry, Status};

use super::models;

pub struct SearchEvent<'a> {
    request: &'a models::Search,
    start: SystemTime,
}

impl<'a> SearchEvent<'a> {
    pub fn new(request: &'a models::Search) -> Self {
        Self {
            request,
            start: SystemTime::now(),
        }
    }

    pub fn log(self, response: &[GameEntry]) {
        info!(
            http_request.request_method = "POST",
            http_request.request_url = "/search",
            labels.log_type = QUERY_LOGS,
            labels.handler = SEARCH_HANDLER,
            request.title = self.request.title,
            response.candidates = response.len(),
            search.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "search '{}'",
            self.request.title
        )
    }

    pub fn log_error(self, status: Status) {
        error!(
            http_request.request_method = "POST",
            http_request.request_url = "/search",
            labels.log_type = QUERY_LOGS,
            labels.handler = SEARCH_HANDLER,
            labels.status = status.to_string(),
            request.title = self.request.title,
            search.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "search '{}'",
            self.request.title
        )
    }
}

pub struct ResolveEvent<'a> {
    request: &'a models::Resolve,
    start: SystemTime,
}

impl<'a> ResolveEvent<'a> {
    pub fn new(request: &'a models::Resolve) -> Self {
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
        match (&self.request.game_entry, &self.request.unmatch_entry) {
            (Some(_), None) => "match",
            (None, Some(_)) => "unmatch",
            (Some(_), Some(_)) => "rematch",
            (None, None) => "bad_request",
        }
    }
}

const QUERY_LOGS: &str = "query_logs";
const SEARCH_HANDLER: &str = "search";
const RESOLVE_HANDLER: &str = "resolve";
const MATCH_HANDLER: &str = "match";
