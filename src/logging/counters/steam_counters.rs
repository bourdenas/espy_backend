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

    pub fn log(self, name: &str) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = STEAM,
            counter.name = "fetch",
            counter.latency = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis(),
            "Steam fetch '{}'",
            name,
        )
    }

    pub fn log_error(&self, appid: &str, error: &str, status: &Status) {
        info!(
            labels.log_type = COUNTERS,
            counter.group = STEAM,
            counter.name = error,
            counter.status = status.to_string(),
            "Steam {error} appid={appid}",
        )
    }
}

const COUNTERS: &str = "counters";
const STEAM: &str = "steam";
