use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{
    documents::{GameDigest, GameEntry},
    log_event,
    logging::LogEvent,
    Status,
};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct ResolveEvent {
    method: String,
    request: Request,
    response: Response,
}

impl ResolveEvent {
    pub fn retrieve(id: u64, response: &Result<GameEntry, Status>) {
        log_event!(LogEvent::Resolve(ResolveEvent {
            method: "retrieve".to_owned(),
            request: Request::Id(id),
            response: match response {
                Ok(game_entry) => Response::Success(game_entry.name.clone()),
                Err(status) => Response::Error(status.to_string()),
            },
        }))
    }

    pub fn resolve(id: u64, response: &Result<GameEntry, Status>) {
        log_event!(LogEvent::Resolve(ResolveEvent {
            method: "resolve".to_owned(),
            request: Request::Id(id),
            response: match response {
                Ok(game_entry) => Response::Success(game_entry.name.clone()),
                Err(status) => Response::Error(status.to_string()),
            },
        }))
    }

    pub fn digest(id: u64, response: &Result<GameDigest, Status>) {
        log_event!(LogEvent::Resolve(ResolveEvent {
            method: "digest".to_owned(),
            request: Request::Id(id),
            response: match response {
                Ok(digest) => Response::Success(digest.name.clone()),
                Err(status) => Response::Error(status.to_string()),
            },
        }))
    }

    pub fn search(title: String, response: &Result<Vec<GameDigest>, Status>) {
        log_event!(LogEvent::Resolve(ResolveEvent {
            method: "search".to_owned(),
            request: Request::Title(title),
            response: match response {
                Ok(digests) => Response::Search(digests.len()),
                Err(status) => Response::Error(status.to_string()),
            },
        }))
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
enum Request {
    Id(u64),
    Title(String),
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
enum Response {
    Success(String),
    Search(usize),
    Error(String),
}
