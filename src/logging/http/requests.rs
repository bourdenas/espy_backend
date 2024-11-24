use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::warn;
use valuable::Valuable;

use crate::{
    documents::{Company, GameDigest, GameEntry},
    http::models,
    Status,
};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum LogHttpRequest {
    Invalid,
    Search(SearchRequest, SearchResponse, Status),
    CompanySearch(CompanySearchRequest, CompanySearchResponse, Status),
    Resolve(ResolveRequest, ResolveResponse, Status),
}

impl Default for LogHttpRequest {
    fn default() -> Self {
        LogHttpRequest::Invalid {}
    }
}

impl LogHttpRequest {
    pub fn search(request: models::Search, digests: &[GameDigest]) -> Self {
        LogHttpRequest::Search(
            SearchRequest {
                title: request.title,
                base_game_only: request.base_game_only,
            },
            SearchResponse {
                games: digests
                    .iter()
                    .map(|digest| Document {
                        id: digest.id,
                        name: digest.name.clone(),
                    })
                    .collect(),
            },
            Status::Ok,
        )
    }
    pub fn search_err(request: models::Search, status: Status) -> Self {
        LogHttpRequest::Search(
            SearchRequest {
                title: request.title,
                base_game_only: request.base_game_only,
            },
            SearchResponse::default(),
            status,
        )
    }

    pub fn company_search(request: models::CompanyFetch, companies: &[Company]) -> Self {
        LogHttpRequest::CompanySearch(
            CompanySearchRequest {
                name: request.name.clone(),
            },
            CompanySearchResponse {
                companies: companies
                    .iter()
                    .map(|company| Document {
                        id: company.id,
                        name: company.name.clone(),
                    })
                    .collect(),
            },
            Status::Ok,
        )
    }

    pub fn company_search_err(request: models::CompanyFetch, status: Status) -> Self {
        LogHttpRequest::CompanySearch(
            CompanySearchRequest { name: request.name },
            CompanySearchResponse::default(),
            status,
        )
    }

    pub fn resolve(request: models::Resolve, game_entry: GameEntry) -> Self {
        LogHttpRequest::Resolve(
            ResolveRequest {
                id: request.game_id,
            },
            ResolveResponse {
                game: Some(Document {
                    id: game_entry.id,
                    name: game_entry.name,
                }),
            },
            Status::Ok,
        )
    }

    pub fn resolve_err(request: models::Resolve, status: Status) -> Self {
        LogHttpRequest::Resolve(
            ResolveRequest {
                id: request.game_id,
            },
            ResolveResponse::default(),
            status,
        )
    }

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

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
pub struct SearchRequest {
    title: String,
    base_game_only: bool,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
pub struct SearchResponse {
    games: Vec<Document>,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
pub struct CompanySearchRequest {
    name: String,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
pub struct CompanySearchResponse {
    companies: Vec<Document>,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
pub struct ResolveRequest {
    id: u64,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
pub struct ResolveResponse {
    game: Option<Document>,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
struct Document {
    id: u64,
    name: String,
}
