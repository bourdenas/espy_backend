use std::time::SystemTime;

use tracing::info;

use crate::Status;

pub struct SteamFetchCounter {
    start: SystemTime,
}

impl SteamFetchCounter {
    pub fn new() -> Self {
        Self {
            start: SystemTime::now(),
        }
    }

    pub fn log(self) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = STEAM,
            counter.name = "fetch",
            counter.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "Steam fetch",
        )
    }

    pub fn log_warning(&self, warning: &str, status: &Status) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = STEAM,
            counter.name = warning,
            counter.status = status.to_string(),
            "Steam warning: {warning}",
        )
    }

    pub fn log_error(self, status: &Status) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = STEAM,
            counter.name = "fetch_fail",
            counter.status = status.to_string(),
            "Steam fetch fail",
        )
    }
}

const COUNTERS: &str = "counters";
const STEAM: &str = "steam";
