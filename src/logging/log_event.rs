use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::warn;
use valuable::Valuable;

use super::{DiffEvent, FirestoreEvent, MetacriticEvent, RejectEvent, ResolveEvent, SteamEvent};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum LogEvent {
    Invalid,
    Firestore(FirestoreEvent),
    Resolve(ResolveEvent),
    Filter(RejectEvent),
    Diff(DiffEvent),
    Steam(SteamEvent),
    Metacritic(MetacriticEvent),
}

impl Default for LogEvent {
    fn default() -> Self {
        LogEvent::Invalid {}
    }
}

impl LogEvent {
    pub fn encode(&self) -> String {
        match serde_json::to_string(self) {
            Ok(json) => json,
            Err(e) => {
                warn!("{}", e);
                String::default()
            }
        }
    }
}

#[macro_export]
macro_rules! log_event {
    ($event:expr) => {
        ::tracing::debug!(event = $event.encode())
    };
}
