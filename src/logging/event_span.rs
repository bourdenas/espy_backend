use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::warn;
use valuable::Valuable;

use super::{FirestoreEvent, ResolveEvent};

#[derive(Serialize, Deserialize, Valuable, Default, Clone, Debug)]
pub struct EventSpan {
    pub name: &'static str,

    #[serde(default)]
    pub latency: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<EventSpan>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<LogEvent>,
}

impl EventSpan {
    pub fn new(name: &'static str) -> Self {
        EventSpan {
            name,
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum LogEvent {
    Invalid,
    Firestore(FirestoreEvent),
    Resolve(ResolveEvent),
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
macro_rules! log {
    ($event:expr) => {
        ::tracing::debug!(event = $event.encode());
    };
}
