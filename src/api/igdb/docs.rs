use serde::Deserialize;

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

#[derive(Deserialize, Default, Debug, Clone)]
pub struct IgdbGame {
    pub id: u64,
    pub name: String,

    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub summary: String,

    #[serde(default)]
    pub storyline: String,

    #[serde(default)]
    pub first_release_date: Option<i64>,

    #[serde(default)]
    pub aggregated_rating: Option<f64>,

    #[serde(default)]
    pub total_rating: Option<f64>,

    #[serde(default)]
    pub follows: i64,

    #[serde(default)]
    pub hypes: i64,

    #[serde(default)]
    pub category: u64,

    #[serde(default)]
    pub genres: Vec<u64>,

    #[serde(default)]
    pub keywords: Vec<u64>,

    #[serde(default)]
    pub expansions: Vec<u64>,

    #[serde(default)]
    pub standalone_expansions: Vec<u64>,

    #[serde(default)]
    pub dlcs: Vec<u64>,

    #[serde(default)]
    pub remakes: Vec<u64>,

    #[serde(default)]
    pub remasters: Vec<u64>,

    #[serde(default)]
    pub bundles: Vec<u64>,

    #[serde(default)]
    pub platforms: Vec<u64>,

    #[serde(default)]
    pub parent_game: Option<u64>,

    #[serde(default)]
    pub version_parent: Option<u64>,

    #[serde(default)]
    pub version_title: Option<String>,

    #[serde(default)]
    pub collection: Option<u64>,

    #[serde(default)]
    pub franchise: Option<u64>,

    #[serde(default)]
    pub franchises: Vec<u64>,

    #[serde(default)]
    pub involved_companies: Vec<u64>,

    #[serde(default)]
    pub cover: Option<u64>,

    #[serde(default)]
    pub screenshots: Vec<u64>,

    #[serde(default)]
    pub artworks: Vec<u64>,

    #[serde(default)]
    pub websites: Vec<u64>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct IgdbExternalGame {
    pub id: u64,
    pub game: u64,
    pub uid: String,

    #[serde(default)]
    pub url: Option<String>,
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
