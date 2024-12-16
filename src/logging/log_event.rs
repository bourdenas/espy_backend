use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::warn;
use valuable::Valuable;

use super::{DiffEvent, FirestoreEvent, MetacriticEvent, RejectEvent, ResolveEvent, SteamEvent};

#[derive(Serialize, Deserialize, Valuable, Debug)]
pub enum LogEvent {
    Firestore(FirestoreEvent),
    Reject(RejectEvent),
    Diff(DiffEvent),
    Resolve(ResolveEvent),
    Steam(SteamEvent),
    Metacritic(MetacriticEvent),
}

impl LogEvent {
    pub fn encode(&self) -> String {
        match serde_json::to_string(self) {
            Ok(json) => json,
            Err(e) => {
                warn!("{}", e);
                String::default()
            }
        }
    }
}

#[macro_export]
macro_rules! log_event {
    ($event:expr) => {
        ::tracing::debug!(event = $event.encode())
    };
}

#[derive(Serialize, Deserialize, Valuable, Default, Debug)]
pub struct SpanEvents {
    firestore: Option<FirestoreEvent>,
    reject: Option<RejectEvent>,
    diff: Option<DiffEvent>,
    resolver: Option<ResolveEvent>,
    steam: Option<SteamEvent>,
    metacritic: Option<MetacriticEvent>,
}

impl SpanEvents {
    pub fn add(&mut self, event: LogEvent) {
        match event {
            LogEvent::Firestore(firestore_event) => match &mut self.firestore {
                Some(firestore) => firestore.merge(firestore_event),
                None => self.firestore = Some(firestore_event),
            },
            LogEvent::Reject(reject_event) => self.reject = Some(reject_event),
            LogEvent::Diff(diff_event) => self.diff = Some(diff_event),
            LogEvent::Resolve(resolve_event) => self.resolver = Some(resolve_event),
            LogEvent::Steam(steam_event) => match &mut self.steam {
                Some(steam) => {
                    if let Some(event) = steam_event.get_owned_games {
                        steam.get_owned_games = Some(event);
                    }
                    if let Some(event) = steam_event.get_app_details {
                        steam.get_app_details = Some(event);
                    }
                    if let Some(event) = steam_event.get_app_score {
                        steam.get_app_score = Some(event);
                    }
                    if let Some(event) = steam_event.scrape_app_page {
                        steam.scrape_app_page = Some(event);
                    }
                }
                None => self.steam = Some(steam_event),
            },
            LogEvent::Metacritic(metacritic_event) => self.metacritic = Some(metacritic_event),
        }
    }
}
