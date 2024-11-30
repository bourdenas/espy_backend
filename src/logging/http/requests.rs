use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{
    documents::{Company, GameDigest, GameEntry, StoreEntry},
    http::models,
    log_request,
    logging::LogRequest,
    Status,
};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum LogHttpRequest {
    Search {
        request: SearchRequest,
        response: SearchResponse,
        status: Status,
    },
    CompanySearch {
        request: CompanySearchRequest,
        response: CompanySearchResponse,
        status: Status,
    },
    Resolve {
        request: ResolveRequest,
        response: ResolveResponse,
        status: Status,
    },
    Update {
        request: UpdateRequest,
        status: Status,
    },
    Match {
        request: MatchRequest,
        status: Status,
    },
    Wishlist {
        request: WishlistRequest,
        status: Status,
    },
    Unlink {
        request: UnlinkRequest,
        status: Status,
    },
    Sync {
        status: Status,
    },
}

impl LogHttpRequest {
    pub fn search(request: models::Search, digests: &[GameDigest]) {
        log_request!(LogRequest::Http(LogHttpRequest::Search {
            request: SearchRequest {
                title: request.title,
                base_game_only: request.base_game_only,
            },
            response: SearchResponse {
                games: digests
                    .iter()
                    .map(|digest| Document {
                        id: digest.id,
                        name: digest.name.clone(),
                    })
                    .collect(),
            },
            status: Status::Ok,
        }))
    }
    pub fn search_err(request: models::Search, status: Status) {
        log_request!(LogRequest::Http(LogHttpRequest::Search {
            request: SearchRequest {
                title: request.title,
                base_game_only: request.base_game_only,
            },
            response: SearchResponse::default(),
            status,
        }))
    }

    pub fn company_search(request: models::CompanyFetch, companies: &[Company]) {
        log_request!(LogRequest::Http(LogHttpRequest::CompanySearch {
            request: CompanySearchRequest {
                name: request.name.clone(),
            },
            response: CompanySearchResponse {
                companies: companies
                    .iter()
                    .map(|company| Document {
                        id: company.id,
                        name: company.name.clone(),
                    })
                    .collect(),
            },
            status: Status::Ok,
        }))
    }

    pub fn company_search_err(request: models::CompanyFetch, status: Status) {
        log_request!(LogRequest::Http(LogHttpRequest::CompanySearch {
            request: CompanySearchRequest { name: request.name },
            response: CompanySearchResponse::default(),
            status,
        }))
    }

    pub fn resolve(request: models::Resolve, game_entry: GameEntry) {
        log_request!(LogRequest::Http(LogHttpRequest::Resolve {
            request: ResolveRequest {
                id: request.game_id,
            },
            response: ResolveResponse {
                game: Some(Document {
                    id: game_entry.id,
                    name: game_entry.name,
                }),
            },
            status: Status::Ok,
        }))
    }

    pub fn resolve_err(request: models::Resolve, status: Status) {
        log_request!(LogRequest::Http(LogHttpRequest::Resolve {
            request: ResolveRequest {
                id: request.game_id,
            },
            response: ResolveResponse::default(),
            status,
        }))
    }

    pub fn update(request: models::UpdateOp, status: Status) {
        log_request!(LogRequest::Http(LogHttpRequest::Update {
            request: UpdateRequest {
                id: request.game_id,
            },
            status,
        }))
    }

    pub fn unlink(request: models::Unlink, status: Status) {
        log_request!(LogRequest::Http(LogHttpRequest::Unlink {
            request: UnlinkRequest {
                storefront_id: request.storefront_id,
            },
            status,
        }))
    }

    pub fn sync(status: Status) {
        log_request!(LogRequest::Http(LogHttpRequest::Sync { status }))
    }

    pub fn match_game(request: models::MatchOp, status: Status) {
        log_request!(LogRequest::Http(LogHttpRequest::Match {
            request: MatchRequest {
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
        }))
    }

    pub fn wishlist(request: models::WishlistOp, status: Status) {
        log_request!(LogRequest::Http(LogHttpRequest::Wishlist {
            request: WishlistRequest {
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
        }))
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
