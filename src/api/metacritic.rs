use soup::prelude::*;

use crate::Status;

pub struct MetacriticApi {}

impl MetacriticApi {
    pub async fn get_score(slug: &str) -> Result<u64, Status> {
        let uri = format!("https://www.metacritic.com/game/{slug}/");

        let resp = reqwest::get(&uri).await?;
        let text = resp.text().await?;
        let soup = Soup::new(&text);

        match soup.class(SCORE_TAG).find() {
            Some(score_tag) => match score_tag.tag("span").find() {
                Some(span) => match span.text().parse() {
                    Ok(num) => Ok(num),
                    Err(_) => Err(Status::not_found(format!("Missing score for {slug}"))),
                },
                None => Err(Status::not_found(format!("Score not found for {slug}"))),
            },
            None => Err(Status::not_found(format!("{slug} not found"))),
        }
    }

    pub fn guess_id(igdb_url: &str) -> &str {
        igdb_url.split('/').last().unwrap_or("")
    }
}

const SCORE_TAG: &str = "c-productScoreInfo_scoreNumber";
