use serde::{de::DeserializeOwned, Serialize};

use crate::{
    documents::{GameDigest, GameEntry, IgdbGame},
    logging::ResolveEvent,
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
        let response = post(&format!("{}/retrieve", &self.url), id).await;
        ResolveEvent::retrieve(id, &response);
        response
    }

    pub async fn resolve(&self, igdb_game: IgdbGame) -> Result<GameEntry, Status> {
        let id = igdb_game.id;
        let response = post(&format!("{}/resolve", &self.url), igdb_game).await;
        ResolveEvent::resolve(id, &response);
        response
    }

    pub async fn digest(&self, id: u64) -> Result<GameDigest, Status> {
        let response = post(&format!("{}/digest", &self.url), id).await;
        ResolveEvent::digest(id, &response);
        response
    }

    pub async fn search(
        &self,
        title: String,
        base_game_only: bool,
    ) -> Result<Vec<GameDigest>, Status> {
        let response = post(
            &format!("{}/search", &self.url),
            SearchRequest {
                title: title.clone(),
                base_game_only,
            },
        )
        .await;

        ResolveEvent::search(title, &response);
        response
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
