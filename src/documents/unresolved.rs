use serde::{Deserialize, Serialize};

use super::{GameDigest, StoreEntry};

/// Document type under 'users/{user_id}/games/unresolved' with StoreEntries
/// that were not resolved automatically..
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct UnresolvedEntries {
    pub need_approval: Vec<Unresolved>,
    pub unknown: Vec<StoreEntry>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Unresolved {
    pub store_entry: StoreEntry,
    pub candidates: Vec<GameDigest>,
}
