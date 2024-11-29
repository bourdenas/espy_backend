use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::warn;
use valuable::Valuable;

use super::{LogEvent, LogHttpRequest, LogWebhooksRequest};

#[derive(Serialize, Deserialize, Valuable, Default, Clone, Debug)]
pub struct EventSpan {
    pub name: &'static str,

    #[serde(default)]
    pub latency: u64,

    pub request: LogHttpRequest,

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

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum LogRequest {
    Http(LogHttpRequest),
    Webhooks(LogWebhooksRequest),
}

impl LogRequest {
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
