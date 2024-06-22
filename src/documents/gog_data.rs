use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct GogData {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critic_score: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl GogData {
    pub fn release_timestamp(&self) -> Option<i64> {
        match &self.release_date {
            Some(date) => {
                let parsed_date = NaiveDateTime::parse_from_str(
                    &format!("{} 12:00:00", &date),
                    "%B %e, %Y %H:%M:%S",
                );
                match parsed_date {
                    Ok(date) => Some(date.timestamp()),
                    Err(_) => {
                        let parsed_date = NaiveDateTime::parse_from_str(
                            &format!("{} 12:00:00", &date),
                            "%e %B, %Y %H:%M:%S",
                        );
                        match parsed_date {
                            Ok(date) => Some(date.timestamp()),
                            Err(_) => None,
                        }
                    }
                }
            }
            None => None,
        }
    }
}
