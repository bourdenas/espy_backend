use crate::documents;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Search {
    pub title: String,

    #[serde(default)]
    pub base_game_only: bool,
}

impl std::fmt::Display for Search {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CompanyFetch {
    pub name: String,
}

impl std::fmt::Display for CompanyFetch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Resolve {
    pub game_id: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MatchOp {
    pub user_id: String,

    /// The storefront entry that is {un}matched.
    pub store_entry: documents::StoreEntry,

    /// A IGDB game id to match the storefront entry with, if one is provided.
    #[serde(default)]
    pub game_id: Option<u64>,

    /// The library entry that the storefront entry will be unmatched from, if
    /// one is provided. The library entry will be also be deleted from the
    /// library if it contains no other storefront entry.
    #[serde(default)]
    pub unmatch_entry: Option<documents::LibraryEntry>,

    /// If true, deletes the store_entry from the library. Otherwise, it moves
    /// the store_entry to the failed-to-match collection, unless a rematch is
    /// provided.
    #[serde(default)]
    pub delete_unmatched: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct UpdateOp {
    pub user_id: String,
    pub game_id: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WishlistOp {
    pub user_id: String,

    #[serde(default)]
    pub add_game: Option<documents::LibraryEntry>,

    #[serde(default)]
    pub remove_game: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Unlink {
    pub user_id: String,

    pub storefront_id: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Sync {
    pub user_id: String,
}
