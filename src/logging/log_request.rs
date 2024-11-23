use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::warn;
use valuable::Valuable;

use super::{CompanySearchRequest, CompanySearchResponse, SearchRequest, SearchResponse};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum LogRequest {
    Invalid,
    HttpSearch(SearchRequest),
    HttpCompanySearch(CompanySearchRequest),
}

impl Default for LogRequest {
    fn default() -> Self {
        LogRequest::Invalid {}
    }
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

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum LogResponse {
    Invalid,
    HttpSearch(SearchResponse),
    HttpCompanySearch(CompanySearchResponse),
}

impl Default for LogResponse {
    fn default() -> Self {
        LogResponse::Invalid {}
    }
}

impl LogResponse {
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
macro_rules! log_request {
    ($request:expr) => {
        ::tracing::debug!(request = $request.encode());
    };
}

#[macro_export]
macro_rules! log_response {
    ($response:expr) => {
        ::tracing::debug!(response = $response.encode());
    };
}
