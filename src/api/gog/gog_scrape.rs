use std::collections::HashSet;

use reqwest::{header, ClientBuilder};
use soup::prelude::*;

use crate::{documents::GogData, Status};

pub struct GogScrape {}

impl GogScrape {
    pub async fn scrape(url: &str) -> Result<GogData, Status> {
        let mut request_headers = header::HeaderMap::new();
        request_headers.insert(
            header::ACCEPT_LANGUAGE,
            header::HeaderValue::from_static("en-US;en"),
        );

        let client = ClientBuilder::new()
            .default_headers(request_headers)
            .cookie_store(true)
            .build()
            .unwrap();

        // Uncomment for forcing english page from GOG, but it issues
        // two http requests.
        let resp = client
            .get("https://www.gog.com/user/changeLanguage/en")
            .send()
            .await?;
        resp.text().await?;

        let resp = client.get(url).send().await?;
        let text = resp.text().await?;
        let soup = Soup::new(&text);

        let description = match soup.class(DESCRIPTION).find() {
            Some(description) => Some(description.display()),
            None => None,
        };

        let logo = match soup.class(LOGO).find() {
            Some(img) => match img.get("srcset") {
                Some(srcset) => extract_logo(&srcset),
                None => None,
            },
            None => None,
        };

        let mut release_date = None;
        for div in soup.class(DETAILS_ROW).find_all() {
            if let Some(date) = extract_date(&div.text()) {
                release_date = Some(date);
                break;
            }
        }

        let mut genres = HashSet::new();
        let mut tags = HashSet::new();
        for anchor in soup.class(DETAILS_CELL).find_all() {
            match anchor.get("href") {
                Some(href) => {
                    if let Some(genre) = extract_genre(&href) {
                        genres.insert(genre);
                    }
                    if let Some(tag) = extract_tag(&href) {
                        tags.insert(tag);
                    }
                }
                None => {}
            }
        }

        let critic_score = match soup.class(CRITICS_RATING_WRAPPER).find() {
            Some(div) => match div.class(CRITICS_SCORE).find() {
                Some(span) => extract_score(&span.text()),
                None => None,
            },
            None => None,
        };

        Ok(GogData {
            release_date,
            logo,
            critic_score,
            genres: genres.into_iter().collect(),
            tags: tags.into_iter().collect(),
            description,
        })
    }
}

use lazy_static::lazy_static;
use regex::Regex;

fn extract_logo(input: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<url>https:[\w\/\.\-\_]+\.(png|jpg))").unwrap();
    }
    RE.captures(input)
        .and_then(|cap| cap.name("url").map(|url| url.as_str().to_owned()))
}

fn extract_date(text: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<date>\d+\-\d+\-\d+)T").unwrap();
    }
    RE.captures(text)
        .and_then(|cap| cap.name("date").map(|url| url.as_str().to_owned()))
}

fn extract_genre(input: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"\/games\?genres=(?P<genre>[\w\_\-]+)").unwrap();
    }
    RE.captures(input)
        .and_then(|cap| cap.name("genre").map(|url| url.as_str().to_owned()))
}

fn extract_tag(input: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"\/games\/tags/(?P<tag>[\w\_\-]+)").unwrap();
    }
    RE.captures(input)
        .and_then(|cap| cap.name("tag").map(|url| url.as_str().to_owned()))
}

fn extract_score(input: &str) -> Option<u64> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<score>\d+)").unwrap();
    }
    let score = RE.captures(input).and_then(|cap| {
        cap.name("score")
            .map(|url| match url.as_str().parse::<u64>() {
                Ok(score) => score,
                Err(_) => 0,
            })
    });

    match score {
        Some(0) => None,
        Some(x) => Some(x),
        None => None,
    }
}

const LOGO: &str = "productcard-player__logo";
const DETAILS_ROW: &str = "details__row";
const DETAILS_CELL: &str = "details__link";
const CRITICS_RATING_WRAPPER: &str = "critics-rating-wrapper";
const CRITICS_SCORE: &str = "circle-score__text";
const DESCRIPTION: &str = "description";
