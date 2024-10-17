use soup::prelude::*;
use tracing::warn;

#[derive(Default, Clone, Debug)]
pub struct MetacriticData {
    pub score: u64,
    pub review_count: u64,
}

pub struct MetacriticApi {}

impl MetacriticApi {
    pub async fn get_score(slug: &str) -> Option<MetacriticData> {
        let uri = format!("https://www.metacritic.com/game/{slug}/");

        let resp = match reqwest::get(&uri).await {
            Ok(resp) => resp,
            Err(status) => {
                warn!("Failed metacritic request for {slug}: {status}");
                return None;
            }
        };
        let text = match resp.text().await {
            Ok(text) => text,
            Err(status) => {
                warn!("Failed parsing metacritic response for {slug}: {status}");
                return None;
            }
        };
        let soup = Soup::new(&text);

        for tile in soup.class(PLATFORM_TILE).find_all() {
            match tile.tag("title").find() {
                Some(title) => {
                    if title.text() != "PC" {
                        continue;
                    }
                }
                None => continue,
            }

            let review_count = match tile.tag("p").find() {
                Some(reviews_total) => extract_review_count(&reviews_total.text()),
                None => None,
            };

            let score = match tile.class(REVIEWS_SCORE).find() {
                Some(reviews_score) => match reviews_score.tag("span").find() {
                    Some(span) => match span.text().parse() {
                        Ok(num) => Some(num),
                        Err(_) => None,
                    },
                    None => None,
                },
                None => None,
            };

            if let Some(score) = score {
                return Some(MetacriticData {
                    score,
                    review_count: review_count.unwrap_or_default(),
                });
            }
        }
        None
    }

    pub fn guess_id(igdb_url: &str) -> &str {
        igdb_url.split('/').last().unwrap_or("")
    }
}

use lazy_static::lazy_static;
use regex::Regex;

fn extract_review_count(input: &str) -> Option<u64> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"Based on (?P<count>\d+) Critic Reviews").unwrap();
    }
    RE.captures(input).and_then(|cap| {
        cap.name("count")
            .map(|count| match count.as_str().parse::<u64>() {
                Ok(count) => count,
                Err(_) => 0,
            })
    })
}

const PLATFORM_TILE: &str = "c-gamePlatformTile";
const REVIEWS_SCORE: &str = "c-siteReviewScore";
