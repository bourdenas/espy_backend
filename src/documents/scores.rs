use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Scores {
    // 1-9 tier based on Steam score description.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<u64>,

    // Thumbs up percentage from Steam.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbs: Option<u64>,

    // Popularity measured as total reviews on Steam.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popularity: Option<u64>,

    // Metacritic score sourced either from Steam or IGDB.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metacritic: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub espy_tier: Option<EspyTier>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbs_tier: Option<Thumbs>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pop_tier: Option<Popularity>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critics_tier: Option<Critics>,
}

impl Scores {
    pub fn calculate_tiers(&mut self) {
        if let Some(pop) = self.popularity {
            self.pop_tier = Some(Popularity::create(pop));
        }
        if let Some(thumbs) = self.thumbs {
            self.thumbs_tier = Some(Thumbs::create(thumbs));
        }
        if let Some(critics) = self.metacritic {
            self.critics_tier = Some(Critics::create(critics));
        }
        self.espy_tier = Some(EspyTier::create(&self));
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EspyTier {
    Masterpiece,
    Excellent,
    Great,
    VeryGood,
    Ok,
    Mixed,
    Flop,
    NotGood,
    Bad,
    Unknown,
}

impl EspyTier {
    pub fn create(scores: &Scores) -> Self {
        match (&scores.thumbs_tier, &scores.pop_tier) {
            (Some(thumb), Some(pop)) => match thumb {
                Thumbs::Masterpiece => match pop {
                    Popularity::Massive | Popularity::Hit => match scores.critics_tier {
                        Some(Critics::Masterpiece) | Some(Critics::Excellent) => Self::Masterpiece,
                        _ => Self::Excellent,
                    },
                    Popularity::Popular => Self::Excellent,
                    Popularity::Niche => Self::Great,
                    _ => Self::Unknown,
                },
                Thumbs::Excellent => match pop {
                    Popularity::Massive | Popularity::Hit | Popularity::Popular => Self::Excellent,
                    Popularity::Niche => Self::Great,
                    _ => Self::Unknown,
                },
                Thumbs::Great => match pop {
                    Popularity::Massive | Popularity::Hit | Popularity::Popular => Self::Great,
                    Popularity::Niche => Self::Great,
                    _ => Self::Unknown,
                },
                Thumbs::VeryGood => match pop {
                    Popularity::Fringe => Self::Unknown,
                    _ => Self::VeryGood,
                },
                Thumbs::Good => match pop {
                    Popularity::Fringe => Self::Unknown,
                    _ => Self::Ok,
                },
                Thumbs::Mixed => Self::Mixed,
                Thumbs::NotGood => match pop {
                    Popularity::Massive | Popularity::Hit | Popularity::Popular => Self::Flop,
                    _ => Self::NotGood,
                },
                Thumbs::Bad => match pop {
                    Popularity::Massive | Popularity::Hit | Popularity::Popular => Self::Flop,
                    _ => Self::Bad,
                },
                _ => Self::Unknown,
            },

            _ => Self::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Thumbs {
    Masterpiece = 95,
    Excellent = 90,
    Great = 85,
    VeryGood = 80,
    Good = 70,
    Mixed = 50,
    NotGood = 40,
    Bad = 0,
    Unknown,
}

impl Thumbs {
    pub fn create(count: u64) -> Self {
        match count {
            x if x >= Self::Masterpiece as u64 => Self::Masterpiece,
            x if x >= Self::Excellent as u64 => Self::Excellent,
            x if x >= Self::Great as u64 => Self::Great,
            x if x >= Self::VeryGood as u64 => Self::VeryGood,
            x if x >= Self::Good as u64 => Self::Good,
            x if x >= Self::Mixed as u64 => Self::Mixed,
            x if x >= Self::NotGood as u64 => Self::NotGood,
            x if x > Self::Bad as u64 => Self::Bad,
            _ => Self::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Popularity {
    Massive = 100000,
    Hit = 20000,
    Popular = 5000,
    Niche = 1000,
    Fringe = 0,
    Unknown,
}

impl Popularity {
    pub fn create(count: u64) -> Self {
        match count {
            x if x >= Self::Massive as u64 => Self::Massive,
            x if x >= Self::Hit as u64 => Self::Hit,
            x if x >= Self::Popular as u64 => Self::Popular,
            x if x >= Self::Niche as u64 => Self::Niche,
            x if x > Self::Fringe as u64 => Self::Fringe,
            _ => Self::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Critics {
    Masterpiece = 95,
    Excellent = 90,
    Great = 85,
    VeryGood = 80,
    Good = 70,
    Mixed = 50,
    NotGood = 40,
    Bad = 0,
    Unknown,
}

impl Critics {
    pub fn create(count: u64) -> Self {
        match count {
            x if x >= Self::Masterpiece as u64 => Self::Masterpiece,
            x if x >= Self::Excellent as u64 => Self::Excellent,
            x if x >= Self::Great as u64 => Self::Great,
            x if x >= Self::VeryGood as u64 => Self::VeryGood,
            x if x >= Self::Good as u64 => Self::Good,
            x if x >= Self::Mixed as u64 => Self::Mixed,
            x if x >= Self::NotGood as u64 => Self::NotGood,
            x if x > Self::Bad as u64 => Self::Bad,
            _ => Self::Unknown,
        }
    }
}
