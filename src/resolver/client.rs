use serde::{de::DeserializeOwned, Serialize};

use crate::{
    api::IgdbGame,
    documents::{GameDigest, GameEntry},
    Status,
};

use super::models::SearchRequest;

#[derive(Clone)]
pub struct ResolveApi {
    url: String,
}

impl ResolveApi {
    pub fn new(url: String) -> Self {
        ResolveApi { url }
    }

    pub async fn retrieve(&self, id: u64) -> Result<GameEntry, Status> {
        let game_entry = post(&format!("{}/retrieve", &self.url), id).await?;
        Ok(game_entry)
    }

    pub async fn resolve(&self, igdb_game: IgdbGame) -> Result<GameEntry, Status> {
        let game_entry = post(&format!("{}/resolve", &self.url), igdb_game).await?;
        Ok(game_entry)
    }

    pub async fn digest(&self, id: u64) -> Result<GameDigest, Status> {
        let digest = post(&format!("{}/digest", &self.url), id).await?;
        Ok(digest)
    }

    pub async fn search(
        &self,
        title: String,
        base_game_only: bool,
    ) -> Result<Vec<GameDigest>, Status> {
        let digest = post(
            &format!("{}/search", &self.url),
            SearchRequest {
                title,
                base_game_only,
            },
        )
        .await?;
        Ok(digest)
    }
}

async fn post<B: Serialize, R: DeserializeOwned>(url: &str, body: B) -> Result<R, Status> {
    let resp = reqwest::Client::new()
        .post(url)
        .body(serde_json::to_string(&body)?)
        .send()
        .await;

    let resp = match resp {
        Ok(resp) => resp,
        Err(e) => {
            let status = Status::internal(format!("Request failed: {e}\nurl: {url}"));
            return Err(status);
        }
    };

    let text = resp.text().await?;
    match serde_json::from_str::<R>(&text) {
        Ok(resp) => Ok(resp),
        Err(_) => {
            let status = Status::internal(format!("Failed to parse response: {text}\nurl: {url}"));
            Err(status)
        }
    }
}
