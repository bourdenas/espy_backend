use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{
    log_event,
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
        log_event!(LogEvent::Filter(RejectEvent::Prefilter(reason)));
    }

    pub fn filter(reason: RejectionReason) {
        log_event!(LogEvent::Filter(RejectEvent::Filter(reason)));
    }

    pub fn exception(reason: RejectionException) {
        log_event!(LogEvent::Filter(RejectEvent::Exception(reason)));
    }
}
