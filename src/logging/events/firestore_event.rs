use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{log_event, logging::LogEvent};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct FirestoreEvent {
    event: Event,
    collection: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    doc: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

impl FirestoreEvent {
    pub fn read(collection: String, doc: String, error: Option<String>) {
        log_event!(LogEvent::Firestore(FirestoreEvent {
            event: Event::Read(ReadStats {
                read: 1,
                not_found: 0,
                criteria: vec![],
            }),
            collection,
            doc: Some(doc),
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
        }))
    }

    pub fn read_not_found(collection: String, doc: String, error: Option<String>) {
        log_event!(LogEvent::Firestore(FirestoreEvent {
            event: Event::Read(ReadStats {
                read: 1,
                not_found: 1,
                criteria: vec![],
            }),
            collection,
            doc: Some(doc),
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
        }))
    }

    pub fn search(
        collection: String,
        criteria: Vec<Criterion>,
        read: usize,
        not_found: usize,
        errors: Vec<String>,
    ) {
        log_event!(LogEvent::Firestore(FirestoreEvent {
            event: Event::Read(ReadStats {
                read,
                not_found,
                criteria,
            }),
            collection,
            doc: None,
            errors,
        }))
    }

    pub fn write(collection: String, doc: String, error: Option<String>) {
        log_event!(LogEvent::Firestore(FirestoreEvent {
            event: Event::Write,
            collection,
            doc: Some(doc),
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
        }))
    }

    pub fn delete(collection: String, doc: String, error: Option<String>) {
        log_event!(LogEvent::Firestore(FirestoreEvent {
            event: Event::Delete,
            collection,
            doc: Some(doc),
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
        }))
    }
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
enum Event {
    Read(ReadStats),
    Write,
    Delete,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
struct ReadStats {
    read: usize,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    not_found: usize,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    criteria: Vec<Criterion>,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct Criterion {
    field: String,
    value: String,
}

impl Criterion {
    pub fn new(field: String, value: String) -> Self {
        Criterion { field, value }
    }
}

fn is_zero(num: &usize) -> bool {
    *num == 0
}
