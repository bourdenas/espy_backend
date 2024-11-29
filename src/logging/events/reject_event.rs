use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use tracing::debug;
use valuable::Valuable;

use crate::{
    logging::LogEvent,
    webhooks::{
        filtering::{RejectionException, RejectionReason},
        prefiltering::PrefilterRejectionReason,
    },
};

#[derive(Serialize, Deserialize, Valuable, Clone, Debug)]
pub enum RejectEvent {
    Prefilter(PrefilterRejectionReason),
    Filter(RejectionReason),
    Exception(RejectionException),
}

impl RejectEvent {
    pub fn prefilter(reason: PrefilterRejectionReason) {
        debug!(event = LogEvent::Filter(RejectEvent::Prefilter(reason)).encode());
    }

    pub fn filter(reason: RejectionReason) {
        debug!(event = LogEvent::Filter(RejectEvent::Filter(reason)).encode());
    }

    pub fn exception(reason: RejectionException) {
        debug!(event = LogEvent::Filter(RejectEvent::Exception(reason)).encode());
    }
}
