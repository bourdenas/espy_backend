use serde::{Deserialize, Serialize};

use super::StoreEntry;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Storefront {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub games: Vec<StoreEntry>,
}
