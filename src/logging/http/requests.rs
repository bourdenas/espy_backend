use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{
    documents::{Company, GameDigest},
    http::models,
    logging::{LogRequest, LogResponse},
    Status,
};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct SearchRequest {
    title: String,
    base_game_only: bool,
}

impl SearchRequest {
    pub fn new(request: &models::Search) -> LogRequest {
        LogRequest::HttpSearch(SearchRequest {
            title: request.title.clone(),
            base_game_only: request.base_game_only,
        })
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct SearchResponse {
    games: Vec<Document>,
    status: Status,
}

impl SearchResponse {
    pub fn new(digests: &[GameDigest]) -> LogResponse {
        LogResponse::HttpSearch(SearchResponse {
            games: digests
                .iter()
                .map(|digest| Document {
                    id: digest.id,
                    name: digest.name.clone(),
                })
                .collect(),
            status: Status::Ok,
        })
    }

    pub fn err(status: Status) -> LogResponse {
        LogResponse::HttpSearch(SearchResponse {
            games: vec![],
            status,
        })
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct CompanySearchRequest {
    name: String,
}

impl CompanySearchRequest {
    pub fn new(request: &models::CompanyFetch) -> LogRequest {
        LogRequest::HttpCompanySearch(CompanySearchRequest {
            name: request.name.clone(),
        })
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct CompanySearchResponse {
    companies: Vec<Document>,
    status: Status,
}

impl CompanySearchResponse {
    pub fn new(companies: &[Company]) -> LogResponse {
        LogResponse::HttpCompanySearch(CompanySearchResponse {
            companies: companies
                .iter()
                .map(|company| Document {
                    id: company.id,
                    name: company.name.clone(),
                })
                .collect(),
            status: Status::Ok,
        })
    }

    pub fn err(status: Status) -> LogResponse {
        LogResponse::HttpSearch(SearchResponse {
            games: vec![],
            status,
        })
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
struct Document {
    id: u64,
    name: String,
}
