use soup::prelude::*;
use tracing::warn;

#[derive(Default, Clone, Debug)]
pub struct SteamScrapeData {
    pub user_tags: Vec<String>,
}

pub struct SteamScrape {}

impl SteamScrape {
    pub async fn scrape(uri: &str) -> Option<SteamScrapeData> {
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
