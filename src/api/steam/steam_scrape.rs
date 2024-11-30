use reqwest::{header, ClientBuilder};
use soup::prelude::*;
use tracing::instrument;

use crate::logging::SteamEvent;

#[derive(Default, Clone, Debug)]
pub struct SteamScrapeData {
    pub user_tags: Vec<String>,
}

pub struct SteamScrape {}

impl SteamScrape {
    #[instrument(name = "steam::scrape_app_page", level = "info")]
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
            Err(e) => {
                SteamEvent::scrape_app_page(url.to_owned(), vec![e.to_string()]);
                return None;
            }
        };
        let text = match resp.text().await {
            Ok(text) => text,
            Err(e) => {
                SteamEvent::scrape_app_page(url.to_owned(), vec![e.to_string()]);
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
