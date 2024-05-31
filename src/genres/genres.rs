use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    documents::{EspyGenre, GameEntry},
    Status,
};

pub struct GenrePredictor;

impl GenrePredictor {
    #[instrument(level = "trace", skip(game_entry))]
    pub async fn annotate(game_entry: &GameEntry) -> Result<Vec<EspyGenre>, Status> {
        let client = reqwest::Client::new();
        let resp = client
            .post(GENRES_PREDICT_URL)
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
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
struct GenrePredictRequest {
    id: u64,
    name: String,
    igdb_genres: Vec<String>,
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
}

const GENRES_PREDICT_URL: &str = "http://localhost:5000/genres";
