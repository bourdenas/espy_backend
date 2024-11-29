use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::debug;
use valuable::Valuable;

use crate::{
    documents::{Company, GameDigest, GameEntry, StoreEntry},
    http::models,
    logging::LogRequest,
    Status,
};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum LogHttpRequest {
    Search(SearchRequest, SearchResponse, Status),
    CompanySearch(CompanySearchRequest, CompanySearchResponse, Status),
    Resolve(ResolveRequest, ResolveResponse, Status),
    Update(UpdateRequest, Status),
    Match(MatchRequest, Status),
    Wishlist(WishlistRequest, Status),
    Unlink(UnlinkRequest, Status),
    Sync(Status),
}

impl LogHttpRequest {
    pub fn search(request: models::Search, digests: &[GameDigest]) {
        debug!(
            request = LogRequest::Http(LogHttpRequest::Search(
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
            ))
            .encode()
        )
    }
    pub fn search_err(request: models::Search, status: Status) {
        debug!(
            request = LogRequest::Http(LogHttpRequest::Search(
                SearchRequest {
                    title: request.title,
                    base_game_only: request.base_game_only,
                },
                SearchResponse::default(),
                status,
            ))
            .encode()
        )
    }

    pub fn company_search(request: models::CompanyFetch, companies: &[Company]) {
        debug!(
            request = LogRequest::Http(LogHttpRequest::CompanySearch(
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
            ))
            .encode()
        )
    }

    pub fn company_search_err(request: models::CompanyFetch, status: Status) {
        debug!(
            request = LogRequest::Http(LogHttpRequest::CompanySearch(
                CompanySearchRequest { name: request.name },
                CompanySearchResponse::default(),
                status,
            ))
            .encode()
        )
    }

    pub fn resolve(request: models::Resolve, game_entry: GameEntry) {
        debug!(
            request = LogRequest::Http(LogHttpRequest::Resolve(
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
            ))
            .encode()
        )
    }

    pub fn resolve_err(request: models::Resolve, status: Status) {
        debug!(
            request = LogRequest::Http(LogHttpRequest::Resolve(
                ResolveRequest {
                    id: request.game_id,
                },
                ResolveResponse::default(),
                status,
            ))
            .encode()
        )
    }

    pub fn update(request: models::UpdateOp, status: Status) {
        debug!(
            request = LogRequest::Http(LogHttpRequest::Update(
                UpdateRequest {
                    id: request.game_id,
                },
                status,
            ))
            .encode()
        )
    }

    pub fn unlink(request: models::Unlink, status: Status) {
        debug!(
            request = LogRequest::Http(LogHttpRequest::Unlink(
                UnlinkRequest {
                    storefront_id: request.storefront_id,
                },
                status,
            ))
            .encode()
        )
    }

    pub fn sync(status: Status) {
        debug!(request = LogRequest::Http(LogHttpRequest::Sync(status)).encode())
    }

    pub fn match_game(request: models::MatchOp, status: Status) {
        debug!(
            request = LogRequest::Http(LogHttpRequest::Match(
                MatchRequest {
                    store_entry: request.store_entry,
                    op: match (request.game_id, request.unmatch_entry) {
                        (Some(id), None) => MatchOp::Match { to: id },
                        (Some(id), Some(library_entry)) => MatchOp::Rematch {
                            from: Document {
                                id: library_entry.id,
                                name: library_entry.digest.name,
                            },
                            to: id,
                        },
                        (None, Some(library_entry)) => MatchOp::Unmatch {
                            from: Document {
                                id: library_entry.id,
                                name: library_entry.digest.name,
                            },
                            delete: request.delete_unmatched,
                        },
                        (None, None) => MatchOp::Invalid,
                    },
                },
                status,
            ))
            .encode()
        )
    }

    pub fn wishlist(request: models::WishlistOp, status: Status) {
        debug!(
            request = LogRequest::Http(LogHttpRequest::Wishlist(
                WishlistRequest {
                    op: match (request.add_game, request.remove_game) {
                        (Some(library_entry), None) => WishlistOp::Add {
                            game: Document {
                                id: library_entry.id,
                                name: library_entry.digest.name,
                            },
                        },
                        (None, Some(id)) => WishlistOp::Remove { id },
                        _ => WishlistOp::Invalid,
                    },
                },
                status,
            ))
            .encode()
        )
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
struct SearchRequest {
    title: String,
    base_game_only: bool,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
struct SearchResponse {
    games: Vec<Document>,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
struct CompanySearchRequest {
    name: String,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
struct CompanySearchResponse {
    companies: Vec<Document>,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
struct ResolveRequest {
    id: u64,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
struct ResolveResponse {
    game: Option<Document>,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
struct UpdateRequest {
    id: u64,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
struct MatchRequest {
    store_entry: StoreEntry,
    op: MatchOp,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
enum MatchOp {
    #[default]
    Invalid,
    Match {
        to: u64,
    },
    Rematch {
        from: Document,
        to: u64,
    },
    Unmatch {
        from: Document,
        delete: bool,
    },
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
struct WishlistRequest {
    op: WishlistOp,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
enum WishlistOp {
    #[default]
    Invalid,
    Add {
        game: Document,
    },
    Remove {
        id: u64,
    },
}

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
struct UnlinkRequest {
    storefront_id: String,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
struct Document {
    id: u64,
    name: String,
}
