use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use super::FirestoreEvent;

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
    InvalidEvent,
    FirestoreEvent(FirestoreEvent),
}

impl Default for LogEvent {
    fn default() -> Self {
        LogEvent::InvalidEvent {}
    }
}
