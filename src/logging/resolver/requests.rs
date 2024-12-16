use std::fmt::{Debug, Display};

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{documents::GameDigest, log_request, logging::LogRequest, resolver::models, Status};

#[derive(Serialize, Deserialize, Valuable, Debug)]
pub enum LogResolverRequest {
    Retrieve {
        id: u64,
        resolved: Option<String>,
        status: Status,
    },
    Resolve {
        id: u64,
        resolved: Option<String>,
        status: Status,
    },
    Digest {
        id: u64,
        resolved: Option<String>,
        status: Status,
    },
    Search {
        request: SearchRequest,
        response: SearchResponse,
        status: Status,
    },
}

impl LogResolverRequest {
    pub fn retrieve(id: u64, resolved: Option<String>, status: Status) {
        log_request!(LogRequest::Resolver(LogResolverRequest::Retrieve {
            id,
            resolved,
            status
        }))
    }

    pub fn resolve(id: u64, resolved: Option<String>, status: Status) {
        log_request!(LogRequest::Resolver(LogResolverRequest::Resolve {
            id,
            resolved,
            status
        }))
    }

    pub fn digest(id: u64, resolved: Option<String>, status: Status) {
        log_request!(LogRequest::Resolver(LogResolverRequest::Digest {
            id,
            resolved,
            status
        }))
    }

    pub fn search(request: models::SearchRequest, results: &[GameDigest], status: Status) {
        log_request!(LogRequest::Resolver(LogResolverRequest::Search {
            request: SearchRequest {
                title: request.title,
                base_game_only: request.base_game_only
            },
            response: SearchResponse {
                games: results
                    .iter()
                    .map(|digest| Document {
                        id: digest.id,
                        name: digest.name.clone()
                    })
                    .collect()
            },
            status,
        }))
    }
}

impl Display for LogResolverRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogResolverRequest::Retrieve {
                id,
                resolved,
                status: _,
            } => write!(
                f,
                "retrieve {id} --> {}",
                match resolved {
                    Some(title) => title,
                    None => "failed",
                }
            ),
            LogResolverRequest::Resolve {
                id,
                resolved,
                status: _,
            } => write!(
                f,
                "resolve {id} --> {}",
                match resolved {
                    Some(title) => title,
                    None => "failed",
                }
            ),
            LogResolverRequest::Digest {
                id,
                resolved,
                status: _,
            } => write!(
                f,
                "digest {id} --> {}",
                match resolved {
                    Some(title) => title,
                    None => "failed",
                }
            ),
            LogResolverRequest::Search {
                request,
                response: _,
                status: _,
            } => {
                write!(f, "search '{}'", request.title)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Valuable, Default, Debug)]
pub struct SearchRequest {
    title: String,
    base_game_only: bool,
}

#[derive(Serialize, Deserialize, Valuable, Default, Debug)]
pub struct SearchResponse {
    games: Vec<Document>,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
struct Document {
    id: u64,
    name: String,
}
