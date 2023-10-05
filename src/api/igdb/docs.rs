use std::{collections::HashSet, fmt};

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
    pub category: u64,

    #[serde(default)]
    pub status: u64,

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follows: Option<u64>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hypes: Option<u64>,

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
        self.follows.unwrap_or_default() > 0 || self.hypes.unwrap_or_default() > 0
    }

    pub fn diff(&self, other: &IgdbGame) -> IgdbGameDiff {
        IgdbGameDiff {
            name: self.name != other.name,
            category: self.category != other.category,
            status: self.status != other.status,

            url: self.url != other.url,
            summary: self.summary != other.summary,
            storyline: self.storyline != other.storyline,

            first_release_date: self.first_release_date != other.first_release_date,
            aggregated_rating: self.aggregated_rating != other.aggregated_rating,

            follows: self.follows != other.follows,
            hypes: self.hypes != other.hypes,

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

            parent_game: self.parent_game != other.parent_game,
            version_parent: self.version_parent != other.version_parent,
            version_title: self.version_title != other.version_title,

            collection: self.collection != other.collection,
            franchise: self.franchise != other.franchise,
            franchises: vec_diff(&self.franchises, &other.franchises),
            involved_companies: vec_diff(&self.involved_companies, &other.involved_companies),

            cover: self.cover != other.cover,
            screenshots: vec_diff(&self.screenshots, &other.screenshots),
            artworks: vec_diff(&self.artworks, &other.artworks),
            websites: vec_diff(&self.websites, &other.websites),
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
    #[serde(default, skip_serializing_if = "is_default")]
    pub name: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub category: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub status: bool,

    #[serde(default, skip_serializing_if = "is_default")]
    pub url: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub summary: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub storyline: bool,

    #[serde(default, skip_serializing_if = "is_default")]
    pub first_release_date: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub aggregated_rating: bool,

    #[serde(default, skip_serializing_if = "is_default")]
    pub follows: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub hypes: bool,

    #[serde(default, skip_serializing_if = "is_default")]
    pub genres: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub keywords: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub expansions: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub standalone_expansions: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub dlcs: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub remakes: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub remasters: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub bundles: bool,

    #[serde(default, skip_serializing_if = "is_default")]
    pub parent_game: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub version_parent: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub version_title: bool,

    #[serde(default, skip_serializing_if = "is_default")]
    pub collection: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub franchise: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub franchises: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub involved_companies: bool,

    #[serde(default, skip_serializing_if = "is_default")]
    pub cover: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub screenshots: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub artworks: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub websites: bool,
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
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
        self.name
            || self.category
            || self.status
            || self.url
            || self.summary
            || self.storyline
            || self.first_release_date
            || self.aggregated_rating
            || self.follows
            || self.hypes
            || self.genres
            || self.keywords
            || self.expansions
            || self.standalone_expansions
            || self.dlcs
            || self.remakes
            || self.remasters
            || self.bundles
            || self.parent_game
            || self.version_parent
            || self.version_title
            || self.collection
            || self.franchise
            || self.franchises
            || self.involved_companies
            || self.cover
            || self.screenshots
            || self.artworks
            || self.websites
    }

    pub fn needs_resolve(&self) -> bool {
        self.genres
            || self.keywords
            || self.expansions
            || self.standalone_expansions
            || self.dlcs
            || self.remakes
            || self.remasters
            || self.parent_game
            || self.version_parent
            || self.collection
            || self.franchise
            || self.franchises
            || self.involved_companies
            || self.cover
            || self.screenshots
            || self.artworks
            || self.websites
    }
}

fn vec_diff(left: &[u64], right: &[u64]) -> bool {
    let left = HashSet::<u64>::from_iter(left.iter().cloned());
    let right = HashSet::<u64>::from_iter(right.iter().cloned());
    left.intersection(&right).count() != left.len()
}
