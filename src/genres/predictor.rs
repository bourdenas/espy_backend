use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    documents::{EspyGenre, GameEntry},
    Status,
};

pub struct GenrePredictor {
    url: String,
}

impl GenrePredictor {
    pub fn new(url: String) -> Self {
        GenrePredictor { url }
    }

    #[instrument(level = "trace", skip(self, game_entry))]
    pub async fn predict(&self, game_entry: &GameEntry) -> Result<Vec<EspyGenre>, Status> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/genres", &self.url))
            .json(&GenrePredictRequest::new(game_entry))
            .send()
            .await?;

        let text = resp.text().await?;
        let resp = serde_json::from_str::<GenrePredictResponse>(&text).map_err(|e| {
            Status::internal(format!(
                "Parse error: {e}\n GenrePredictor response: {text}"
            ))
        })?;

        Ok(resp
            .espy_genres
            .iter()
            .map(|label| EspyGenre::from(label.as_str()))
            .collect())
    }

    #[instrument(level = "trace", skip(self, game_entry))]
    pub async fn debug(&self, game_entry: &GameEntry) -> Result<GenreDebugInfo, Status> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/genres_debug", &self.url))
            .json(&GenrePredictRequest::new(game_entry))
            .send()
            .await?;

        let text = resp.text().await?;
        let resp = serde_json::from_str::<GenrePredictResponse>(&text).map_err(|e| {
            Status::internal(format!(
                "Parse error: {e}\n GenrePredictor response: {text}"
            ))
        })?;

        Ok(resp.debug_info)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
struct GenrePredictRequest {
    id: u64,
    name: String,
    igdb_genres: Vec<String>,
    igdb_keywords: Vec<String>,
    steam_genres: Vec<String>,
    steam_tags: Vec<String>,
}

impl GenrePredictRequest {
    fn new(game_entry: &GameEntry) -> Self {
        GenrePredictRequest {
            id: game_entry.id,
            name: game_entry.name.clone(),
            igdb_genres: game_entry
                .igdb_genres
                .iter()
                .map(|genre| format!("{:?}", genre))
                .collect(),
            igdb_keywords: game_entry.keywords.clone(),
            steam_genres: match &game_entry.steam_data {
                Some(steam_data) => steam_data
                    .genres
                    .iter()
                    .map(|e| e.description.clone())
                    .collect(),
                None => vec![],
            },
            steam_tags: match &game_entry.steam_data {
                Some(steam_data) => steam_data.user_tags.clone(),
                None => vec![],
            },
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
struct GenrePredictResponse {
    id: u64,
    name: String,
    espy_genres: Vec<String>,

    #[serde(default)]
    debug_info: GenreDebugInfo,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct GenreDebugInfo {
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

const _GENRES_PREDICT_URL: &str = "https://genrelearner-fjxkoqq4wq-ew.a.run.app/genres";
const _GENRES_DEBUG_URL: &str = "http://localhost:8080/genres_debug";
