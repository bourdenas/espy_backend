use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::warn;
use valuable::Valuable;

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct FirestoreEvent {
    op: Op,
    collection: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    doc: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

impl FirestoreEvent {
    pub fn read(collection: String, doc: String, error: Option<String>) -> Self {
        FirestoreEvent {
            op: Op::Read(ReadStats {
                read: 1,
                not_found: 0,
            }),
            collection,
            doc: Some(doc),
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
        }
    }

    pub fn read_not_found(collection: String, doc: String, error: Option<String>) -> Self {
        FirestoreEvent {
            op: Op::Read(ReadStats {
                read: 1,
                not_found: 1,
            }),
            collection,
            doc: Some(doc),
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
        }
    }

    pub fn batch(collection: String, num: usize, not_found: usize, errors: Vec<String>) -> Self {
        FirestoreEvent {
            op: Op::Read(ReadStats {
                read: num,
                not_found,
            }),
            collection,
            doc: None,
            errors,
        }
    }

    pub fn write(collection: String, doc: String, error: Option<String>) -> Self {
        FirestoreEvent {
            op: Op::Write,
            collection,
            doc: Some(doc),
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
        }
    }

    pub fn delete(collection: String, doc: String, error: Option<String>) -> Self {
        FirestoreEvent {
            op: Op::Delete,
            collection,
            doc: Some(doc),
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
        }
    }

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

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
enum Op {
    Read(ReadStats),
    Write,
    Delete,
}

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
struct ReadStats {
    read: usize,

    #[serde(skip_serializing_if = "is_zero")]
    not_found: usize,
}

fn is_zero(num: &usize) -> bool {
    *num == 0
}
