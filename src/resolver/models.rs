use serde::{Deserialize, Serialize};

use crate::documents::{GameEntry, IgdbGame};

use super::igdb::filtering::RejectionReason;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SearchRequest {
    pub title: String,

    #[serde(default)]
    pub base_game_only: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ResolveRequest {
    pub igdb_game: IgdbGame,

    /// If true, resolved GameEntry will be filtered-out if it is not deemed a
    /// significant release. A rejection reason is returned in the response.
    #[serde(default)]
    pub filter: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ResolveResponse {
    Success(GameEntry),
    Reject(RejectionReason),
}
