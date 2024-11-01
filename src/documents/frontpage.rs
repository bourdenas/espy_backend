use serde::{Deserialize, Serialize};

use super::{GameDigest, ReleaseEvent};

/// Document for 'espy/frontpage' that contains content for building espy frontpage.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Frontpage {
    #[serde(default)]
    pub last_updated: u64,

    // Subset of recent/upcoming releases of the timeline that is included
    // directly in the frontpage. Games released last or next 30 days.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub timeline: Vec<ReleaseEvent>,

    // Games released today.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub today_releases: Vec<GameDigest>,

    // Games released in the next X weeks.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub upcoming_releases: Vec<GameDigest>,

    // Games released in the past X weeks.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_releases: Vec<GameDigest>,

    // Future games that got a release date recently.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_release_dates: Vec<GameDigest>,

    // Future games that got announced recently.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_announcements: Vec<GameDigest>,

    // Most hyped upcoming games.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hyped: Vec<GameDigest>,
}
