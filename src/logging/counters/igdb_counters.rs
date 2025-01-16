use std::time::SystemTime;

use tracing::info;

use crate::{documents::GameEntry, resolver::filtering::RejectionReason, Status};

pub struct IgdbCounters;

impl IgdbCounters {
    pub fn connection_fail(status: &Status) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = IGDB,
            counter.name = "connection_fail",
            counter.status = status.to_string(),
            "IGDB connection failed",
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
            counter.group = IGDB,
            counter.name = "resolve",
            counter.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "IGDB resolve: '{}' ({})",
            &game_entry.name,
            &game_entry.id,
        )
    }

    pub fn log_reject(self, rejection: &RejectionReason) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = IGDB,
            counter.name = "resolve_reject",
            counter.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "IGDB resolve rejected: {:?}",
            &rejection.reason,
        )
    }

    pub fn log_error(self, status: &Status) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = IGDB,
            counter.name = "resolve_fail",
            counter.status = status.to_string(),
            "IGDB resolve failed",
        )
    }
}

pub struct IgdbRequestCounter<'a> {
    request: &'a str,
    _start: SystemTime,
}

impl<'a> IgdbRequestCounter<'a> {
    pub fn new(request: &'a str) -> Self {
        Self {
            request,
            _start: SystemTime::now(),
        }
    }

    pub fn log(self) {
        // info!(
        //     labels.log_type = COUNTERS,
        //     counter.group = IGDB,
        //     counter.name = self.request,
        //     counter.latency = SystemTime::now()
        //         .duration_since(self.start)
        //         .unwrap()
        //         .as_millis(),
        //     "IGDB request: {}",
        //     self.request,
        // );
    }

    pub fn log_error(self, status: &Status) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = IGDB,
            counter.name = &format!("{}_fail", self.request),
            counter.status = status.to_string(),
            "IGDB request failed: {}",
            self.request,
        )
    }
}

const COUNTERS: &str = "counters";
const IGDB: &str = "igdb";
