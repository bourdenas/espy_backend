use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::{GogData, IgdbExternalGame};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExternalGame {
    pub igdb_id: u64,
    pub store_id: String,

    #[serde(default)]
    pub name: String,
    pub store_name: StoreName,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_url: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gog_data: Option<GogData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum StoreName {
    Steam,
    Gog,
    Egs,
    Other(u64),
}

impl Display for StoreName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreName::Steam => write!(f, "steam"),
            StoreName::Gog => write!(f, "gog"),
            StoreName::Egs => write!(f, "egs"),
            StoreName::Other(id) => write!(f, "other({id})"),
        }
    }
}

impl From<u64> for StoreName {
    fn from(id: u64) -> Self {
        match id {
            1 => StoreName::Steam,
            5 => StoreName::Gog,
            26 => StoreName::Egs,
            id => StoreName::Other(id),
        }
    }
}

impl From<IgdbExternalGame> for ExternalGame {
    fn from(external: IgdbExternalGame) -> Self {
        ExternalGame {
            igdb_id: external.game,
            store_id: external.uid,
            name: external.name,
            store_name: StoreName::from(external.category),
            store_url: external.url,
            gog_data: None,
        }
    }
}
