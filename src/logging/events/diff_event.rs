use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{documents::IgdbGameDiff, log_event, logging::LogEvent};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct DiffEvent {
    diff: String,
    needs_resolve: bool,
}

impl DiffEvent {
    pub fn diff(diff: &IgdbGameDiff) {
        log_event!(LogEvent::Diff(DiffEvent {
            diff: diff.to_string(),
            needs_resolve: diff.needs_resolve()
        }));
    }
}
