use serde::{Deserialize, Serialize};

use crate::api::IgdbExternalGame;

use super::GogData;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ExternalGame {
    pub igdb_id: u64,
    pub store_id: String,

    pub store_name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_url: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gog_data: Option<GogData>,
}

impl ExternalGame {
    pub fn is_steam(&self) -> bool {
        self.store_name == "steam"
    }

    pub fn is_gog(&self) -> bool {
        self.store_name == "gog"
    }

    pub fn is_egs(&self) -> bool {
        self.store_name == "egs"
    }
}

impl From<IgdbExternalGame> for ExternalGame {
    fn from(external: IgdbExternalGame) -> Self {
        ExternalGame {
            store_name: external.store().to_owned(),
            igdb_id: external.game,
            store_id: external.uid,
            store_url: external.url,

            ..Default::default()
        }
    }
}
