use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use serde::{Deserialize, Serialize};
use tracing::warn;
use valuable::Valuable;

use super::{LogHttpRequest, LogWebhooksRequest, SpanEvents};

#[derive(Serialize, Deserialize, Valuable, Default, Debug)]
pub struct EventSpan {
    pub name: &'static str,

    pub latency: u64,

    pub latencies: HashMap<String, u64>,

    pub request: LogRequest,

    pub events: SpanEvents,

    pub errors: Vec<String>,
}

impl EventSpan {
    pub fn new(name: &'static str) -> Self {
        EventSpan {
            name,
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Valuable, Default, Debug)]
pub enum LogRequest {
    #[default]
    None,

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

impl Display for LogRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogRequest::Http(request) => write!(f, "{request}"),
            LogRequest::Webhooks(request) => write!(f, "{request}"),
            LogRequest::None => write!(f, "None"),
        }
    }
}

#[macro_export]
macro_rules! log_request {
    ($request:expr) => {
        ::tracing::debug!(request = $request.encode())
    };
}

#[macro_export]
macro_rules! log_error {
    ($status:expr) => {
        ::tracing::error!(error = $status.to_string())
    };
}
