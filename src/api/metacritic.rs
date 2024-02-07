use soup::prelude::*;

use crate::Status;

pub struct MetacriticApi {}

impl MetacriticApi {
    pub async fn get_score(slug: &str, year: i32) -> Result<u64, Status> {
        let uri = format!("https://www.metacritic.com/game/{slug}/");

        let resp = reqwest::get(&uri).await?;
        let text = resp.text().await?;
        let soup = Soup::new(&text);

        match soup.class(PLATFORM_LOGO).find() {
            Some(platform) => match platform.tag("title").find() {
                Some(title) => {
                    println!("{}", title.text());
                    if title.text() != "PC" {
                        return Err(Status::not_found(format!("Score not found for {slug}")));
                    }
                }
                None => return Err(Status::not_found(format!("Score not found for {slug}"))),
            },
            None => return Err(Status::not_found(format!("Score not found for {slug}"))),
        }

        match soup.class(SCORE_CONTENT).find() {
            Some(content) => {
                if year > 2010 {
                    let count = match content.class(SCORE_REVIEWS_TOTAL).find() {
                        Some(reviews_total) => match reviews_total.tag("span").find() {
                            Some(span) => extract_review_count(&span.text()),
                            None => None,
                        },
                        None => None,
                    };

                    if count.unwrap_or(0) < 10 {
                        return Err(Status::not_found(format!("Score not found for {slug}")));
                    }
                }

                match content.class(SCORE_NUMBER).find() {
                    Some(score_number) => match score_number.tag("span").find() {
                        Some(span) => match span.text().parse() {
                            Ok(num) => Ok(num),
                            Err(_) => Err(Status::not_found(format!("Score not found for {slug}"))),
                        },
                        None => Err(Status::not_found(format!("Score not found for {slug}"))),
                    },
                    None => Err(Status::not_found(format!("Score not found for {slug}"))),
                }
            }
            None => Err(Status::not_found(format!("Score not found for {slug}"))),
        }
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

const PLATFORM_LOGO: &str = "c-gamePlatformLogo";
const SCORE_CONTENT: &str = "c-productScoreInfo_scoreContent";
const SCORE_REVIEWS_TOTAL: &str = "c-productScoreInfo_reviewsTotal";
const SCORE_NUMBER: &str = "c-productScoreInfo_scoreNumber";
