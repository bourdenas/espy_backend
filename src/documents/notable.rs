use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Notable {
    #[serde(default)]
    pub companies: Vec<String>,

    // Companies to preserve for classic games (prior to 2011).
    #[serde(default)]
    pub legacy_companies: Vec<String>,

    // Collections to preserve for classic games (prior to 2011).
    #[serde(default)]
    pub collections: Vec<String>,

    #[serde(default)]
    pub last_updated: u64,
}
