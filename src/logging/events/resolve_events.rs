use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{
    documents::{GameDigest, GameEntry},
    logging::LogEvent,
    Status,
};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum ResolveEvent {
    Retrieve(Request),
    Resolve(Request),
    Digest(Request),
    Search(SearchRequest),
}

impl ResolveEvent {
    pub fn retrieve(id: u64, response: &Result<GameEntry, Status>) -> LogEvent {
        LogEvent::Resolve(ResolveEvent::Retrieve(Request {
            id,
            result: match response {
                Ok(game_entry) => Response::Success(game_entry.name.clone()),
                Err(status) => Response::Error(status.to_string()),
            },
        }))
    }

    pub fn resolve(id: u64, response: &Result<GameEntry, Status>) -> LogEvent {
        LogEvent::Resolve(ResolveEvent::Resolve(Request {
            id,
            result: match response {
                Ok(game_entry) => Response::Success(game_entry.name.clone()),
                Err(status) => Response::Error(status.to_string()),
            },
        }))
    }

    pub fn digest(id: u64, response: &Result<GameDigest, Status>) -> LogEvent {
        LogEvent::Resolve(ResolveEvent::Digest(Request {
            id,
            result: match response {
                Ok(digest) => Response::Success(digest.name.clone()),
                Err(status) => Response::Error(status.to_string()),
            },
        }))
    }

    pub fn search(title: String, response: &Result<Vec<GameDigest>, Status>) -> LogEvent {
        LogEvent::Resolve(ResolveEvent::Search(SearchRequest {
            title,
            result: match response {
                Ok(digests) => SearchResponse::Success(
                    digests.iter().map(|digest| digest.name.clone()).collect(),
                ),
                Err(status) => SearchResponse::Error(status.to_string()),
            },
        }))
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct Request {
    id: u64,
    result: Response,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
enum Response {
    Success(String),
    Error(String),
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct SearchRequest {
    title: String,
    result: SearchResponse,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
enum SearchResponse {
    Success(Vec<String>),
    Error(String),
}
