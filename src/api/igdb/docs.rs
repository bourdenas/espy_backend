use std::fmt;

use serde::{Deserialize, Serialize};

use crate::documents::Image;

#[derive(Deserialize, Default, Debug, Clone)]
pub struct IgdbGameShort {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub first_release_date: Option<i64>,

    #[serde(default)]
    pub aggregated_rating: Option<f64>,

    #[serde(default)]
    pub category: u64,

    #[serde(default)]
    pub version_parent: Option<u64>,

    #[serde(default)]
    pub platforms: Vec<u64>,

    #[serde(default)]
    pub cover: Option<Image>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct IgdbGame {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub url: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub summary: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub storyline: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_release_date: Option<i64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregated_rating: Option<f64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_rating: Option<f64>,

    #[serde(default)]
    pub follows: i64,

    #[serde(default)]
    pub hypes: i64,

    #[serde(default)]
    pub category: u64,

    #[serde(default)]
    pub status: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub expansions: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub standalone_expansions: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dlcs: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remakes: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remasters: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bundles: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_game: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_parent: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_title: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub franchise: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub franchises: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub involved_companies: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artworks: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub websites: Vec<u64>,
}

impl IgdbGame {
    pub fn is_pc_game(&self) -> bool {
        self.platforms.contains(&6) || self.platforms.contains(&13) || self.platforms.contains(&14)
    }

    pub fn has_hype(&self) -> bool {
        self.follows > 0 || self.hypes > 0
    }

    pub fn diff(&self, other: &IgdbGame) -> IgdbGameDiff {
        IgdbGameDiff {
            id: self.id,
            name: match self.name == other.name {
                false => Some(other.name.clone()),
                true => None,
            },
            category: match self.category == other.category {
                false => Some(other.category),
                true => None,
            },
            status: match self.status == other.status {
                false => Some(other.status),
                true => None,
            },

            url: match self.url == other.url {
                false => Some(other.url.clone()),
                true => None,
            },
            summary: match self.summary == other.summary {
                false => Some(other.summary.clone()),
                true => None,
            },
            storyline: match self.storyline == other.storyline {
                false => Some(other.storyline.clone()),
                true => None,
            },
            first_release_date: match self.first_release_date == other.first_release_date {
                false => other.first_release_date,
                true => None,
            },
            aggregated_rating: match self.aggregated_rating == other.aggregated_rating {
                false => other.aggregated_rating,
                true => None,
            },

            parent_game: match self.parent_game == other.parent_game {
                false => other.parent_game,
                true => None,
            },
            version_parent: match self.version_parent == other.version_parent {
                false => other.version_parent,
                true => None,
            },
            version_title: match self.version_title == other.version_title {
                false => other.version_title.clone(),
                true => None,
            },

            cover: match self.cover == other.cover {
                false => other.cover,
                true => None,
            },
            collection: match self.collection == other.collection {
                false => other.collection,
                true => None,
            },
            franchise: match self.franchise == other.franchise {
                false => other.franchise,
                true => None,
            },

            franchises: vec_diff(&self.franchises, &other.franchises),
            involved_companies: vec_diff(&self.involved_companies, &other.involved_companies),
            screenshots: vec_diff(&self.screenshots, &other.screenshots),
            artworks: vec_diff(&self.artworks, &other.artworks),
            websites: vec_diff(&self.websites, &other.websites),

            genres: vec_diff(&self.genres, &other.genres),
            keywords: vec_diff(&self.keywords, &other.keywords),
            expansions: vec_diff(&self.expansions, &other.expansions),
            standalone_expansions: vec_diff(
                &self.standalone_expansions,
                &other.standalone_expansions,
            ),
            dlcs: vec_diff(&self.dlcs, &other.dlcs),
            remakes: vec_diff(&self.remakes, &other.remakes),
            remasters: vec_diff(&self.remasters, &other.remasters),
            bundles: vec_diff(&self.bundles, &other.bundles),
        }
    }
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct IgdbExternalGame {
    pub id: u64,
    pub game: u64,
    pub uid: String,

    // Enum of the external's game store.
    // {steam: 1, GOG: 5}
    pub category: u64,

    #[serde(default)]
    pub url: Option<String>,
}

impl IgdbExternalGame {
    pub fn is_steam(&self) -> bool {
        self.category == 1
    }

    pub fn is_gog(&self) -> bool {
        self.category == 5
    }

    pub fn store(&self) -> &str {
        match self.category {
            1 => "steam",
            5 => "gog",
            _ => "unknown",
        }
    }
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct IgdbInvolvedCompany {
    pub id: u64,

    #[serde(default)]
    pub company: Option<u64>,

    #[serde(default)]
    pub developer: bool,

    #[serde(default)]
    pub publisher: bool,

    #[serde(default)]
    pub porting: bool,

    #[serde(default)]
    pub supporting: bool,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct IgdbCompany {
    pub id: u64,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub slug: String,

    #[serde(default)]
    pub logo: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub developed: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub published: Vec<u64>,
}

#[derive(Deserialize, Default, Debug)]
pub struct IgdbCollection {
    pub id: u64,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub slug: String,

    #[serde(default)]
    pub url: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub games: Vec<u64>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct IgdbWebsite {
    pub id: u64,
    pub category: i32,
    pub url: String,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct IgdbAnnotation {
    pub id: u64,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub slug: String,
}

#[derive(Serialize, Default, Debug, Clone)]
pub struct IgdbGameDiff {
    pub id: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storyline: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_release_date: Option<i64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregated_rating: Option<f64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_game: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_parent: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_title: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub franchise: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub franchises: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub involved_companies: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artworks: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub websites: Vec<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub expansions: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub standalone_expansions: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dlcs: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remakes: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remasters: Vec<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bundles: Vec<u64>,
}

impl fmt::Display for IgdbGameDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string(&self) {
            Ok(text) => write!(f, "{text}"),
            Err(_) => write!(f, "IgdbGame diff failed"),
        }
    }
}

impl IgdbGameDiff {
    pub fn empty(&self) -> bool {
        !self.is_not_empty()
    }

    pub fn is_not_empty(&self) -> bool {
        self.name.is_some()
            || self.category.is_some()
            || self.status.is_some()
            || self.url.is_some()
            || self.summary.is_some()
            || self.storyline.is_some()
            || self.first_release_date.is_some()
            || self.aggregated_rating.is_some()
            || self.parent_game.is_some()
            || self.version_parent.is_some()
            || self.version_title.is_some()
            || self.cover.is_some()
            || self.collection.is_some()
            || self.franchise.is_some()
            || !self.franchises.is_empty()
            || !self.involved_companies.is_empty()
            || !self.screenshots.is_empty()
            || !self.artworks.is_empty()
            || !self.websites.is_empty()
            || !self.genres.is_empty()
            || !self.keywords.is_empty()
            || !self.expansions.is_empty()
            || !self.standalone_expansions.is_empty()
            || !self.dlcs.is_empty()
            || !self.remakes.is_empty()
            || !self.remasters.is_empty()
            || !self.bundles.is_empty()
    }

    pub fn needs_resolve(&self) -> bool {
        self.parent_game.is_some()
            || self.version_parent.is_some()
            || self.cover.is_some()
            || self.collection.is_some()
            || self.franchise.is_some()
            || !self.franchises.is_empty()
            || !self.involved_companies.is_empty()
            || !self.screenshots.is_empty()
            || !self.artworks.is_empty()
            || !self.websites.is_empty()
            || !self.genres.is_empty()
            || !self.keywords.is_empty()
            || !self.expansions.is_empty()
            || !self.standalone_expansions.is_empty()
            || !self.dlcs.is_empty()
            || !self.remakes.is_empty()
            || !self.remasters.is_empty()
    }
}

fn vec_diff(left: &[u64], right: &[u64]) -> Vec<u64> {
    match right.is_empty() {
        false => right
            .into_iter()
            .filter(|id| !left.contains(id))
            .cloned()
            .collect(),
        true => vec![],
    }
}
