use reqwest::{header, ClientBuilder};
use soup::prelude::*;
use tracing::warn;

#[derive(Default, Clone, Debug)]
pub struct SteamScrapeData {
    pub user_tags: Vec<String>,
}

pub struct SteamScrape {}

impl SteamScrape {
    pub async fn scrape(url: &str) -> Option<SteamScrapeData> {
        let mut request_headers = header::HeaderMap::new();
        request_headers.insert(
            header::COOKIE,
            header::HeaderValue::from_static("birthtime=0; path=/; max-age=315360000"),
        );

        let client = ClientBuilder::new()
            .default_headers(request_headers)
            .cookie_store(true)
            .build()
            .unwrap();

        let resp = match client.get(url).send().await {
            Ok(resp) => resp,
            Err(status) => {
                warn!("Failed steam scrape request for {url}: {status}");
                return None;
            }
        };
        let text = match resp.text().await {
            Ok(text) => text,
            Err(status) => {
                warn!("Failed to parse steam scrape response for {url}: {status}");
                return None;
            }
        };
        let soup = Soup::new(&text);

        match soup.class(GLANCE_TAGS).find() {
            Some(tags) => Some(SteamScrapeData {
                user_tags: tags
                    .tag("a")
                    .find_all()
                    .map(|tag| tag.text().trim().to_owned())
                    .collect(),
            }),
            None => None,
        }
    }
}

const GLANCE_TAGS: &str = "glance_tags";
