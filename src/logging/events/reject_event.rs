use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{
    documents::{GameCategory, GamePlatform, GameStatus},
    log_event,
    logging::LogEvent,
    resolver::filtering::{Exception, NotableFor, Reason, RejectionException, RejectionReason},
    webhooks::prefiltering::PrefilterRejectionReason,
};

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
pub struct RejectEvent {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    exception: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    platforms: Vec<GamePlatform>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<GameCategory>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<GameStatus>,

    popularity: u64,
    hype: u64,
    year: i32,
}

impl RejectEvent {
    pub fn prefilter(reason: PrefilterRejectionReason) {
        log_event!(LogEvent::Reject(RejectEvent {
            reason: Some(match reason {
                PrefilterRejectionReason::NotPcGame(_) => "prefilter.platform".to_owned(),
                PrefilterRejectionReason::NotMainCategory(_) => "prefilter.category".to_owned(),
                PrefilterRejectionReason::Unknown => "prefilter.unknown".to_owned(),
            }),
            category: match reason {
                PrefilterRejectionReason::NotMainCategory(game_category) => Some(game_category),
                _ => None,
            },
            platforms: match reason {
                PrefilterRejectionReason::NotPcGame(vec) => vec,
                _ => vec![],
            },
            ..Default::default()
        }));
    }

    pub fn filter(reason: RejectionReason) {
        log_event!(LogEvent::Reject(RejectEvent {
            reason: Some(match reason.reason {
                Reason::NoScoreLowPopularity => "filter.metrics".to_owned(),
                Reason::FutureReleaseNoHype => "filter.hype".to_owned(),
                Reason::Unknown => "filter.unknown".to_owned(),
            }),
            category: Some(reason.category),
            status: Some(reason.status),
            popularity: reason.popularity,
            hype: reason.hype,
            year: reason.year,
            ..Default::default()
        }));
    }

    pub fn exception(reason: RejectionException) {
        log_event!(LogEvent::Reject(RejectEvent {
            exception: Some(match reason.exception {
                Exception::Expansion => "exception.expansion".to_owned(),
                Exception::Remaster => "exception.remaster".to_owned(),
                Exception::Notable(notable) => match notable {
                    NotableFor::Developer(dev) => format!("exception.dev.{dev}"),
                    NotableFor::Collection(col) => format!("exception.col.{col}"),
                },
                Exception::GogClassic => "exception.classic".to_owned(),
            }),
            category: Some(reason.category),
            status: Some(reason.status),
            popularity: reason.popularity,
            hype: reason.hype,
            year: reason.year,
            ..Default::default()
        }));
    }
}
