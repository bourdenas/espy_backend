use std::time::Duration;

use tracing::{error, info};

use crate::{documents::GameEntry, Status};

use super::models;

pub struct SearchEvent {
    request: models::Search,
}

impl SearchEvent {
    pub fn new(request: models::Search) -> Self {
        Self { request }
    }

    pub fn log(self, latency: Duration, response: &[GameEntry]) {
        info!(
            http_request.request_method = "POST",
            http_request.request_url = "/search",
            labels.log_type = QUERY_LOGS,
            labels.handler = SEARCH_HANDLER,
            request.title = self.request.title,
            search.latency = latency.as_millis(),
            response.candidates = response.len(),
            "search '{}'",
            self.request.title
        )
    }

    pub fn log_error(self, latency: Duration, status: Status) {
        error!(
            http_request.request_method = "POST",
            http_request.request_url = "/search",
            labels.log_type = QUERY_LOGS,
            labels.handler = SEARCH_HANDLER,
            labels.status = status.to_string(),
            request.title = self.request.title,
            search.latency = latency.as_millis(),
            "search '{}'",
            self.request.title
        )
    }
}

const QUERY_LOGS: &str = "query_logs";
const SEARCH_HANDLER: &str = "search";
