use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::debug;
use valuable::Valuable;

use crate::{documents::IgdbGameDiff, logging::LogEvent};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub struct DiffEvent {
    diff: String,
    needs_resolve: bool,
}

impl DiffEvent {
    pub fn diff(diff: &IgdbGameDiff) {
        debug!(
            event = LogEvent::Diff(DiffEvent {
                diff: diff.to_string(),
                needs_resolve: diff.needs_resolve()
            })
            .encode()
        );
    }
}
