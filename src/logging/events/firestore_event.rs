use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{log_event, logging::LogEvent};

#[derive(Serialize, Deserialize, Valuable, Clone, Default, Debug)]
pub struct FirestoreEvent {
    reads: usize,
    not_found: usize,
    writes: usize,
    deletes: usize,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

impl FirestoreEvent {
    pub fn read(error: Option<String>) {
        log_event!(LogEvent::Firestore(FirestoreEvent {
            reads: 1,
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
            ..Default::default()
        }))
    }

    pub fn read_not_found() {
        log_event!(LogEvent::Firestore(FirestoreEvent {
            not_found: 1,
            ..Default::default()
        }))
    }

    pub fn search(reads: usize, not_found: usize, errors: Vec<String>) {
        log_event!(LogEvent::Firestore(FirestoreEvent {
            reads,
            not_found,
            errors,
            ..Default::default()
        }))
    }

    pub fn write(error: Option<String>) {
        log_event!(LogEvent::Firestore(FirestoreEvent {
            writes: 1,
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
            ..Default::default()
        }))
    }

    pub fn delete(error: Option<String>) {
        log_event!(LogEvent::Firestore(FirestoreEvent {
            deletes: 1,
            errors: match error {
                Some(error) => vec![error],
                None => vec![],
            },
            ..Default::default()
        }))
    }

    pub fn merge(&mut self, mut other: FirestoreEvent) {
        self.reads += other.reads;
        self.not_found += other.not_found;
        self.writes += other.writes;
        self.deletes += other.deletes;
        self.errors.append(&mut other.errors);
    }
}
