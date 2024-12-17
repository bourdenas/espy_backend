use crate::{
    documents::{NewsItem, SteamData, SteamScore, StoreEntry},
    logging::SteamEvent,
    traits::Storefront,
    Status,
};
use async_trait::async_trait;
use reqwest::{header, ClientBuilder};
use std::collections::HashMap;
use tracing::instrument;

pub struct SteamApi {
    steam_key: String,
    steam_user_id: String,
}

impl SteamApi {
    pub fn new(steam_key: &str, steam_user_id: &str) -> SteamApi {
        SteamApi {
            steam_key: String::from(steam_key),
            steam_user_id: String::from(steam_user_id),
        }
    }

    #[instrument(name = "steam::get_app_details", level = "info")]
    pub async fn get_app_details(steam_appid: &str) -> Result<SteamData, Status> {
        let uri =
            format!("https://store.steampowered.com/api/appdetails?appids={steam_appid}&l=english");

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

        let resp = client.get(&uri).send().await?;
        let text = resp.text().await?;
        let (_, resp) = serde_json::from_str::<HashMap<String, SteamAppDetailsResponse>>(&text)
            .map_err(|e| {
                let msg = format!(
                    "Steam /appdetails?appids={steam_appid} parse error: {} in response: {}",
                    e, &text
                );
                let status = Status::request_error(&msg);
                SteamEvent::get_app_details(steam_appid.to_owned(), Some(msg));
                status
            })?
            .into_iter()
            .next()
            .unwrap();

        SteamEvent::get_app_details(steam_appid.to_owned(), None);
        Ok(resp.data)
    }

    #[instrument(name = "steam::get_app_score", level = "info")]
    pub async fn get_app_score(steam_appid: &str) -> Result<SteamScore, Status> {
        let uri = format!("https://store.steampowered.com/appreviews/{steam_appid}?json=1");

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

        let resp = client.get(&uri).send().await?;
        let text = resp.text().await?;
        let resp = serde_json::from_str::<SteamAppReviewsResponse>(&text).map_err(|e| {
            let msg = format!(
                "Steam /appreviews/{steam_appid} parse error: {} in response: {}",
                e, &text
            );
            let status = Status::request_error(&msg);
            SteamEvent::get_app_score(steam_appid.to_owned(), Some(msg));
            status
        })?;

        SteamEvent::get_app_score(steam_appid.to_owned(), None);
        Ok(SteamScore {
            review_score: ((resp.query_summary.total_positive as f64
                / resp.query_summary.total_reviews as f64)
                * 100.0)
                .round() as u64,
            total_reviews: resp.query_summary.total_reviews,
            review_score_desc: resp.query_summary.review_score_desc,
        })
    }

    #[instrument(name = "steam::get_app_news", level = "info")]
    pub async fn get_app_news(steam_appid: &str) -> Result<Vec<NewsItem>, Status> {
        let uri = format!("{STEAM_HOST}{STEAM_GETNEWSFORAPP_SERVICE}?appid={steam_appid}&count=50&maxlength=600&format=json");

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

        let resp = client.get(&uri).send().await?;
        let text = resp.text().await?;
        let resp = serde_json::from_str::<SteamAppNewsResponse>(&text).map_err(|e| {
            let msg = format!(
                "Steam /GetNewsForApp/{steam_appid} parse error: {} in response: {}",
                e, &text
            );
            let status = Status::request_error(&msg);
            SteamEvent::get_app_news(steam_appid.to_owned(), Some(msg));
            status
        })?;

        let mut newsitems = resp.appnews.newsitems;
        newsitems.retain(|item| item.feedname == STEAM_UPDATE_FEEDNAME);

        SteamEvent::get_app_news(steam_appid.to_owned(), None);
        Ok(newsitems)
    }
}

#[async_trait]
impl Storefront for SteamApi {
    fn id() -> String {
        String::from("steam")
    }

    #[instrument(name = "steam::get_owned_games", level = "info", skip(self))]
    async fn get_owned_games(&self) -> Result<Vec<StoreEntry>, Status> {
        let uri = format!(
            "{STEAM_HOST}{STEAM_GETOWNEDGAMES_SERVICE}?key={}&steamid={}&include_appinfo=true&format=json",
            self.steam_key, self.steam_user_id
        );

        let resp = reqwest::get(&uri).await?.json::<SteamResponse>().await;
        match resp {
            Ok(resp) => {
                SteamEvent::get_owned_games(&self.steam_user_id, resp.response.game_count, None);
                Ok(resp
                    .response
                    .games
                    .into_iter()
                    .map(|entry| StoreEntry {
                        id: entry.appid.to_string(),
                        title: entry.name,
                        storefront_name: SteamApi::id(),
                        image: entry.img_icon_url,
                        ..Default::default()
                    })
                    .collect())
            }
            Err(e) => {
                SteamEvent::get_owned_games(&self.steam_user_id, 0, Some(e.to_string()));
                Err(Status::from(e))
            }
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SteamResponse {
    response: GetOwnedGamesResponse,
}

#[derive(Debug, Serialize, Deserialize)]
struct GetOwnedGamesResponse {
    game_count: usize,
    games: Vec<GameEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GameEntry {
    appid: i64,
    name: String,
    playtime_forever: i32,
    img_icon_url: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct SteamAppDetailsResponse {
    success: bool,
    data: SteamData,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct SteamAppReviewsResponse {
    success: u64,
    query_summary: SteamAppReviewsQuerySummary,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct SteamAppReviewsQuerySummary {
    #[serde(default)]
    review_score: u64,

    #[serde(default)]
    review_score_desc: String,

    #[serde(default)]
    total_positive: u64,

    #[serde(default)]
    total_negative: u64,

    #[serde(default)]
    total_reviews: u64,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct SteamAppNewsResponse {
    appnews: SteamAppNews,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct SteamAppNews {
    appid: u64,
    count: usize,

    #[serde(default)]
    newsitems: Vec<NewsItem>,
}

const STEAM_HOST: &str = "http://api.steampowered.com";
const STEAM_GETOWNEDGAMES_SERVICE: &str = "/IPlayerService/GetOwnedGames/v0001/";
const STEAM_GETNEWSFORAPP_SERVICE: &str = "/ISteamNews/GetNewsForApp/v0002/";

const STEAM_UPDATE_FEEDNAME: &str = "steam_community_announcements";
