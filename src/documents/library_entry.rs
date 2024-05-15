use serde::{Deserialize, Serialize};
use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{GameCategory, GameDigest, GameEntry, StoreEntry};

/// Document type under 'users/{user_id}/games/library' that includes user's
/// library with games matched with an IGDB entry.
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Library {
    pub entries: Vec<LibraryEntry>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct LibraryEntry {
    pub id: u64,
    pub digest: GameDigest,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub store_entries: Vec<StoreEntry>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_date: Option<u64>,
}

impl LibraryEntry {
    pub fn new(digest: GameDigest, store_entry: StoreEntry) -> Self {
        LibraryEntry {
            id: digest.id,
            digest,
            store_entries: vec![store_entry],

            added_date: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
        }
    }

    pub fn new_with_expand(game_entry: GameEntry, store_entry: StoreEntry) -> Vec<Self> {
        let mut entries = vec![LibraryEntry::new(
            GameDigest::from(game_entry.clone()),
            store_entry.clone(),
        )];
        entries.extend(
            game_entry
                .contents
                .iter()
                .map(|e| LibraryEntry::new(e.clone(), store_entry.clone())),
        );
        if matches!(game_entry.category, GameCategory::Version) {
            if let Some(parent) = &game_entry.parent {
                if entries.iter().all(|e| e.id != parent.id) {
                    entries.push(LibraryEntry::new(parent.clone(), store_entry.clone()))
                }
            }
        }
        entries
    }
}

impl fmt::Display for LibraryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LibraryEntry({}): '{}'", &self.id, &self.digest.name)
    }
}
