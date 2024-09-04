use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{documents::WikipediaData, Status};

use soup::prelude::*;

pub struct WikipediaScrape {
    keywords: Vec<String>,
}

impl WikipediaScrape {
    pub fn new(kw_source: &str) -> Result<WikipediaScrape, Status> {
        let file = match File::open(kw_source) {
            Ok(file) => file,
            Err(e) => {
                return Err(Status::invalid_argument(format!(
                    "Failed to open file: {e}"
                )));
            }
        };

        let mut keywords = vec![];
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                keywords.push(line);
            }
        }

        Ok(WikipediaScrape { keywords })
    }

    pub async fn scrape(&self, uri: &str) -> Result<WikipediaData, Status> {
        let resp = match reqwest::get(uri).await {
            Ok(resp) => resp,
            Err(e) => {
                return Err(Status::internal(format!(
                    "Failed to reach url `{uri}` with error: {e}"
                )));
            }
        };
        let text = match resp.text().await {
            Ok(text) => text,
            Err(e) => {
                return Err(Status::internal(format!(
                    "Failed to get response error: {e}"
                )));
            }
        };
        let soup = Soup::new(&text);

        let infobox = extract_infobox(&soup);

        Ok(WikipediaData {
            keywords: self.extract_keywords(&soup),
            score: extract_score(&soup),

            developers: infobox
                .iter()
                .filter_map(|row| match row {
                    InfoboxRow::Developer(items) => Some(items),
                    _ => None,
                })
                .flat_map(|v| v)
                .cloned()
                .collect(),

            publishers: infobox
                .iter()
                .filter_map(|row| match row {
                    InfoboxRow::Publisher(items) => Some(items),
                    _ => None,
                })
                .flat_map(|v| v)
                .cloned()
                .collect(),

            genres: infobox
                .iter()
                .filter_map(|row| match row {
                    InfoboxRow::Genre(items) => Some(items),
                    _ => None,
                })
                .flat_map(|v| v)
                .cloned()
                .collect(),
        })
    }

    fn extract_keywords(&self, soup: &Soup) -> Vec<String> {
        let text = soup
            .text()
            .to_lowercase()
            .replace("\n", " ")
            .replace("'", "")
            .replace("\"", "")
            .replace("-", " ");

        let mut keywords = HashSet::new();
        for sentence in text.split(&['.', ',', '!', ';', ':'][..]) {
            let mut ngrams = HashSet::new();

            let tokens: Vec<String> = sentence
                .split_whitespace()
                .map(|word| word.to_string())
                .collect();

            ngrams.extend(tokens.iter().cloned());
            if tokens.len() > 1 {
                for i in 0..tokens.len() - 1 {
                    let bigram = format!("{} {}", tokens[i], tokens[i + 1]);
                    ngrams.insert(bigram);
                }
            }
            if tokens.len() > 2 {
                for i in 0..tokens.len() - 2 {
                    let trigram = format!("{} {} {}", tokens[i], tokens[i + 1], tokens[i + 2]);
                    ngrams.insert(trigram);
                }
            }

            for kw in &self.keywords {
                if ngrams.contains(kw) {
                    keywords.insert(kw.clone());
                }
            }
        }
        keywords.into_iter().collect()
    }
}

#[derive(Debug)]
enum InfoboxRow {
    Developer(Vec<String>),
    Publisher(Vec<String>),
    Genre(Vec<String>),
}

fn extract_infobox(soup: &Soup) -> Vec<InfoboxRow> {
    let mut infobox = vec![];
    if let Some(table) = soup.class(VIDEO_GAME_TABLE).find() {
        for row in table.tag("tr").find_all() {
            let mut xrefs = vec![];
            if let Some(td) = row.tag("td").find() {
                for anchor in td.tag("a").find_all() {
                    xrefs.push(anchor.text());
                }
            }

            if let Some(th) = row.tag("th").find() {
                if let Some(anchor) = th.tag("a").find() {
                    match anchor.get("href").unwrap_or_default().as_str() {
                        GAME_DEVELOPER => infobox.push(InfoboxRow::Developer(xrefs)),
                        GAME_PUBLISHER => infobox.push(InfoboxRow::Publisher(xrefs)),
                        GAME_GENRE => infobox.push(InfoboxRow::Genre(xrefs)),
                        _ => {}
                    }
                }
            }
        }
    }
    infobox
}

fn extract_score(soup: &Soup) -> Option<u64> {
    if let Some(table) = soup.class(AGGREGATORS_TABLE).find() {
        for td in table.tag("td").find_all() {
            // println!("{}", td.text());
            if let Some(score) = parse_score(&td.text()) {
                return Some(score);
            }
        }
    }
    let mut scores = vec![];
    if let Some(table) = soup.class(REVIEWS_TABLE).find() {
        for td in table.tag("td").find_all() {
            // println!("td: {}", td.text());
            if let Some(score) = parse_score(&td.text()) {
                // println!("    {score}");
                scores.push(score);
            } else {
                let span = td.tag("span").attr("role", "img").find();
                if let Some(span) = span {
                    if let Some(title) = span.get("title") {
                        // println!("    {title}");
                        if let Some(score) = parse_stars(&title) {
                            // println!("    {score}");
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
    Some(score)
}

use lazy_static::lazy_static;
use regex::Regex;

fn parse_score(input: &str) -> Option<u64> {
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

fn parse_stars(input: &str) -> Option<u64> {
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

const VIDEO_GAME_TABLE: &str = "ib-video-game";
const GAME_DEVELOPER: &str = "/wiki/Video_game_developer";
const GAME_PUBLISHER: &str = "/wiki/Video_game_publisher";
const GAME_GENRE: &str = "/wiki/Video_game_genre";

const AGGREGATORS_TABLE: &str = "vgr-aggregators";
const REVIEWS_TABLE: &str = "vgr-reviews";
