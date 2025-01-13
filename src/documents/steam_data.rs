use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SteamData {
    pub name: String,
    pub steam_appid: u64,
    pub detailed_description: String,
    pub short_description: String,
    pub about_the_game: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<ReleaseDate>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_image: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_raw: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub developers: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub publishers: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dlc: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<SteamScore>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub news: Vec<NewsItem>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metacritic: Option<Metacritic>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendations: Option<Recommendations>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<Genre>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub user_tags: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<Screenshot>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub movies: Vec<Movie>,
}

impl SteamData {
    pub fn release_timestamp(&self) -> Option<i64> {
        match &self.release_date {
            Some(date) => {
                let parsed_date = NaiveDateTime::parse_from_str(
                    &format!("{} 12:00:00", &date.date),
                    "%b %e, %Y %H:%M:%S",
                );
                match parsed_date {
                    Ok(date) => Some(date.and_utc().timestamp()),
                    Err(_) => {
                        let parsed_date = NaiveDateTime::parse_from_str(
                            &format!("{} 12:00:00", &date.date),
                            "%e %b, %Y %H:%M:%S",
                        );
                        match parsed_date {
                            Ok(date) => Some(date.and_utc().timestamp()),
                            Err(_) => None,
                        }
                    }
                }
            }
            None => None,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ReleaseDate {
    pub coming_soon: bool,
    pub date: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SteamScore {
    #[serde(default)]
    pub review_score: u64,

    #[serde(default)]
    pub total_reviews: u64,

    #[serde(default)]
    pub review_score_desc: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct NewsItem {
    gid: String,
    date: u64,
    pub feedname: String,

    pub url: String,
    pub title: String,
    pub contents: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Metacritic {
    pub score: u64,
    pub url: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Recommendations {
    pub total: u64,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Genre {
    pub id: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Screenshot {
    pub id: u64,
    pub path_thumbnail: String,
    pub path_full: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Movie {
    pub id: u64,
    pub name: String,
    pub thumbnail: String,
    pub webm: WebM,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct WebM {
    pub max: String,
}
