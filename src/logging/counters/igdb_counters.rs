use std::time::SystemTime;

use tracing::info;

use crate::{documents::GameEntry, Status};

use super::counters::*;

pub struct IgdbCounters;

impl IgdbCounters {
    pub fn connection_fail(status: &Status) {
        error_counter(
            "igdb_connection_fail",
            &format!("IGDB connection failed"),
            status,
        )
    }
}

pub struct IgdbResolveCounter {
    start: SystemTime,
}

impl IgdbResolveCounter {
    pub fn new() -> Self {
        Self {
            start: SystemTime::now(),
        }
    }

    pub fn log(self, game_entry: &GameEntry) {
        info!(
            labels.log_type = COUNTERS,
            labels.counter = "igdb_resolve",
            igdb_resolve.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "IGDB resolve: '{}' ({})",
            &game_entry.name,
            &game_entry.id,
        )
    }

    pub fn log_error(self, status: &Status) {
        info!(
            labels.log_type = COUNTERS,
            labels.counter = "igdb_resolve_fail",
            labels.counter_type = "error",
            labels.status = status.to_string(),
            "IGDB resolve failed",
        )
    }
}

pub struct IgdbRequestCounter<'a> {
    request: &'a str,
    start: SystemTime,
}

impl<'a> IgdbRequestCounter<'a> {
    pub fn new(request: &'a str) -> Self {
        Self {
            request,
            start: SystemTime::now(),
        }
    }

    pub fn log(self) {
        info!(
            labels.log_type = COUNTERS,
            labels.counter = self.request,
            igdb_request.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "IGDB request: {}",
            self.request,
        );
    }

    pub fn log_error(self, status: &Status) {
        info!(
            labels.log_type = COUNTERS,
            labels.counter = &format!("{}_fail", self.request),
            labels.counter_type = "error",
            labels.status = status.to_string(),
            "IGDB request failed: {}",
            self.request,
        )
    }
}
