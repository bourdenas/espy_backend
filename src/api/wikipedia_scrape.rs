use soup::prelude::*;
use tracing::warn;

#[derive(Default, Clone, Debug)]
pub struct WikipediaScrapeData {
    pub score: u64,
}

pub struct WikipediaScrape {}

impl WikipediaScrape {
    pub async fn get_score(uri: &str) -> Option<WikipediaScrapeData> {
        let resp = match reqwest::get(uri).await {
            Ok(resp) => resp,
            Err(status) => {
                warn!("{status}");
                return None;
            }
        };
        let text = match resp.text().await {
            Ok(text) => text,
            Err(status) => {
                warn!("{status}");
                return None;
            }
        };
        let soup = Soup::new(&text);

        if let Some(table) = soup.class(AGGREGATORS_TABLE).find() {
            for td in table.tag("td").find_all() {
                println!("{}", td.text());
                if let Some(score) = extract_score(&td.text()) {
                    return Some(WikipediaScrapeData { score });
                }
            }
            return None;
        }

        let mut scores = vec![];
        if let Some(table) = soup.class(REVIEWS_TABLE).find() {
            for td in table.tag("td").find_all() {
                println!("td: {}", td.text());
                if let Some(score) = extract_score(&td.text()) {
                    println!("    {score}");
                    scores.push(score);
                } else {
                    let span = td.tag("span").attr("role", "img").find();
                    if let Some(span) = span {
                        if let Some(title) = span.get("title") {
                            println!("    {title}");
                            if let Some(score) = extract_stars(&title) {
                                println!("    {score}");
                                scores.push(score);
                            }
                        }
                    }
                }
            }
        }

        if scores.len() < 2 {
            return None;
        }

        let total = scores.len() as u64;
        let score = scores.into_iter().reduce(|acc, e| acc + e).unwrap() / total;
        println!("total: {score}");

        Some(WikipediaScrapeData { score })
    }
}

use lazy_static::lazy_static;
use regex::Regex;

fn extract_score(input: &str) -> Option<u64> {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"^(\(PC\) )?(PC: )?((?P<score>\d+\.?\d*)(%|((/|( out of ))(?P<div>\d+)))?)(( \(PC\))|\[)"
        )
        .unwrap();
    }
    let score = RE.captures(input).and_then(|cap| {
        let score = cap
            .name("score")
            .map(|score| match score.as_str().parse::<f64>() {
                Ok(score) => score,
                Err(_) => 0.0,
            });
        let div = cap
            .name("div")
            .map(|score| match score.as_str().parse::<f64>() {
                Ok(score) => score,
                Err(_) => 100.0,
            });

        match (score, div) {
            (Some(score), Some(div)) => Some((score * (100.0 / div)) as u64),
            (Some(score), None) => {
                if score < 10.0 {
                    Some((score * 10.0) as u64)
                } else {
                    Some(score as u64)
                }
            }
            _ => None,
        }
    });
    score
}

fn extract_stars(input: &str) -> Option<u64> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<score>\d+\.?\d*)/(?P<div>\d+) stars").unwrap();
    }
    let score = RE.captures(input).and_then(|cap| {
        let score = cap
            .name("score")
            .map(|score| match score.as_str().parse::<f64>() {
                Ok(score) => score,
                Err(_) => 0.0,
            });
        let div = cap
            .name("div")
            .map(|score| match score.as_str().parse::<f64>() {
                Ok(score) => score,
                Err(_) => 100.0,
            });

        match (score, div) {
            (Some(score), Some(div)) => Some((score * (100.0 / div)) as u64),
            (Some(score), None) => {
                if score < 10.0 {
                    Some((score * 10.0) as u64)
                } else {
                    Some(score as u64)
                }
            }
            _ => None,
        }
    });
    score
}

const AGGREGATORS_TABLE: &str = "vgr-aggregators";
const REVIEWS_TABLE: &str = "vgr-reviews";
