use std::{collections::BTreeMap, fmt::Debug};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct EventSpan {
    pub event: LogEvent,

    #[serde(default)]
    pub latency: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<LogEvent>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub taints: Vec<Taint>,
}

impl EventSpan {
    pub fn create(
        fields: &BTreeMap<String, serde_json::Value>, /*Option<&serde_json::Value>*/
    ) -> Option<Self> {
        match fields.get("event") {
            Some(event) => Some(Self {
                event: match event.as_str().unwrap() {
                    "resolve_event" => LogEvent::ResolveEvent(ResolveEvent {
                        id: fields.get("id").unwrap().as_u64().unwrap(),
                        ..Default::default()
                    }),
                    "igdb_lookup" => LogEvent::IgdbLookup(IgdbLookup::default()),
                    _ => todo!(),
                },
                ..Default::default()
            }),
            None => None,
        }
    }

    pub fn add(&mut self, _span_name: &str, event_span: EventSpan) {
        match &mut self.event {
            LogEvent::ResolveEvent(event) => match event_span.event {
                LogEvent::IgdbLookup(lookup) => event.lookup_game = Some(lookup),
                _ => todo!(),
            },
            LogEvent::ResolveDigestEvent(_) => todo!(),
            LogEvent::ResolveInfoEvent(_) => todo!(),
            LogEvent::SteamFetch(_) => todo!(),
            LogEvent::ResolveLookup(_) => todo!(),
            LogEvent::DigestLookup(_) => todo!(),
            LogEvent::FirestoreOp(_) => todo!(),
            LogEvent::IgdbLookup(_) => todo!(),
            LogEvent::InvalidEvent => panic!("Unexpected InvalidEvent"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum LogEvent {
    InvalidEvent,
    ResolveEvent(ResolveEvent),
    ResolveDigestEvent(ResolveDigestEvent),
    ResolveInfoEvent(ResolveInfoEvent),
    SteamFetch(SteamFetch),
    ResolveLookup(ResolveLookup),
    DigestLookup(DigestLookup),
    FirestoreOp(FirestoreOp),
    IgdbLookup(IgdbLookup),
}

impl Default for LogEvent {
    fn default() -> Self {
        LogEvent::InvalidEvent {}
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct AddGameEvent {
    pub id: u64,
    pub name: String,

    pub resolve: ResolveEvent,

    #[serde(default)]
    pub status: StatusEnum,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ResolveEvent {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lookup_game: Option<IgdbLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_digest: Option<ResolveDigestEvent>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_info: Option<ResolveInfoEvent>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ResolveDigestEvent {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam_fetch: Option<SteamFetch>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<ResolveLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<ResolveLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub franchises: Option<ResolveLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub companies: Option<ResolveLookup>,

    #[serde(default)]
    pub latency: u64,

    #[serde(default)]
    pub status: StatusEnum,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ResolveInfoEvent {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam_fetch: Option<SteamFetch>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<ResolveLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websites: Option<ResolveLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshots: Option<ResolveLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork: Option<ResolveLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<DigestLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansions: Option<DigestLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standalone_expansions: Option<DigestLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dlcs: Option<DigestLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remakes: Option<DigestLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remasters: Option<DigestLookup>,

    #[serde(default)]
    pub latency: u64,

    #[serde(default)]
    pub status: StatusEnum,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SteamFetch {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub taints: Vec<Taint>,

    #[serde(default)]
    pub latency: u64,

    #[serde(default)]
    pub status: StatusEnum,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ResolveLookup {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firestore: Option<FirestoreOp>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub igdb: Vec<IgdbLookup>,

    #[serde(default)]
    pub latency: u64,

    #[serde(default)]
    pub status: StatusEnum,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct DigestLookup {
    pub id: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firestore: Option<FirestoreOp>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub igdb_get: Option<IgdbLookup>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_digest: Option<ResolveDigestEvent>,

    #[serde(default)]
    pub latency: u64,

    #[serde(default)]
    pub status: StatusEnum,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct FirestoreOp {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub reads: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub writes: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub deletes: u64,

    #[serde(default)]
    pub latency: u64,

    #[serde(default)]
    pub status: StatusEnum,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct IgdbLookup;

fn is_zero(x: &u64) -> bool {
    *x == 0
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Taint {
    pub name: String,
    pub error_message: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub enum StatusEnum {
    #[default]
    Ok,

    NotFound,
    Failed,
}
