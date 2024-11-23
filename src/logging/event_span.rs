use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use super::{LogEvent, LogRequest, LogResponse};

#[derive(Serialize, Deserialize, Valuable, Default, Clone, Debug)]
pub struct EventSpan {
    pub name: &'static str,

    #[serde(default)]
    pub latency: u64,

    pub request: LogRequest,
    pub response: LogResponse,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<LogEvent>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<EventSpan>,
}

impl EventSpan {
    pub fn new(name: &'static str) -> Self {
        EventSpan {
            name,
            ..Default::default()
        }
    }
}
