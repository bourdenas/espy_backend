use serde::{Deserialize, Serialize};

/// Document type under 'users/{user_id}/unknown/{entry_id}' that represents
/// user ownership of a title in a storefront that has not yet been matched with
/// an IGDB entry.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct StoreEntry {
    pub id: i64,
    pub title: String,
    pub storefront_name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub image: String,
}