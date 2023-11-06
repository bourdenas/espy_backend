use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct NotableCompanies {
    #[serde(default)]
    pub companies: Vec<String>,

    #[serde(default)]
    pub last_updated: u64,
}
