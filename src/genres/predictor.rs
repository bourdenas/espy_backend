use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use soup::Soup;
use tracing::instrument;

use crate::{
    documents::{EspyGenre, GameEntry, WikipediaData},
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
    pub async fn predict(
        &self,
        game_entry: &GameEntry,
        wiki_data: Option<WikipediaData>,
        parent: Option<&GameEntry>,
        parent_wiki_data: Option<WikipediaData>,
    ) -> Result<Vec<EspyGenre>, Status> {
        let request = GenrePredictRequest::new(game_entry, wiki_data);
        let parent_request = match parent {
            Some(parent) => Some(GenrePredictRequest::new(parent, parent_wiki_data)),
            None => None,
        };

        let request = match (request, parent_request) {
            (request, Some(parent_request)) => {
                match parent_request.signal_size() >= request.signal_size() {
                    true => parent_request,
                    false => request,
                }
            }
            (request, None) => request,
        };

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/genres", &self.url))
            .json(&request)
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
    pub async fn debug(
        &self,
        game_entry: &GameEntry,
        wiki_data: Option<WikipediaData>,
        parent: Option<&GameEntry>,
        parent_wiki_data: Option<WikipediaData>,
    ) -> Result<GenreDebugInfo, Status> {
        let request = GenrePredictRequest::new(game_entry, wiki_data);
        let parent_request = match parent {
            Some(parent) => Some(GenrePredictRequest::new(parent, parent_wiki_data)),
            None => None,
        };

        let request = match (request, parent_request) {
            (request, Some(parent_request)) => {
                match parent_request.signal_size() >= request.signal_size() {
                    true => parent_request,
                    false => request,
                }
            }
            (request, None) => request,
        };

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/genres_debug", &self.url))
            .json(&request)
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
    gog_genres: Vec<String>,
    gog_tags: Vec<String>,
    wiki_genres: Vec<String>,
    wiki_tags: Vec<String>,

    description: String,
}

impl GenrePredictRequest {
    fn new(game_entry: &GameEntry, wiki_data: Option<WikipediaData>) -> Self {
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

            gog_genres: match &game_entry.gog_data {
                Some(gog_data) => gog_data.genres.clone(),
                None => vec![],
            },
            gog_tags: match &game_entry.gog_data {
                Some(gog_data) => gog_data.tags.clone(),
                None => vec![],
            },

            wiki_genres: match &wiki_data {
                Some(wiki_data) => wiki_data.genres.clone(),
                None => vec![],
            },
            wiki_tags: match &wiki_data {
                Some(wiki_data) => wiki_data.keywords.clone(),
                None => vec![],
            },

            description: match &game_entry.steam_data {
                Some(steam_data) => format!(
                    "{} {}",
                    extract_text(&steam_data.about_the_game),
                    extract_text(&steam_data.detailed_description)
                ),
                None => game_entry.igdb_game.summary.replace("\n", " "),
            },
        }
    }

    fn signal_size(&self) -> usize {
        self.igdb_genres.len()
            + self.igdb_keywords.len()
            + self.steam_genres.len()
            + self.steam_tags.len()
            + self.gog_genres.len()
            + self.gog_tags.len()
            + self.wiki_genres.len()
            + self.wiki_tags.len()
    }
}

fn extract_text(html: &str) -> String {
    let soup = Soup::new(&html);
    soup.text().replace("\n", " ")
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
