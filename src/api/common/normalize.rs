use std::collections::HashSet;

use itertools::Itertools;
use lazy_static::lazy_static;
use phf::phf_map;

pub struct CompanyNormalizer;

impl CompanyNormalizer {
    pub fn slug(name: &str) -> String {
        lazy_static! {
            static ref FLUFF_SET: HashSet<String> = FLUFF.iter().map(|e| e.to_string()).collect();
            static ref LOCATION_SET: HashSet<String> =
                LOCATION.iter().map(|e| e.to_string()).collect();
        }

        let name = name.replace(".", "").replace(",", "");

        let tokens: Vec<String> = name
            .split_whitespace()
            .map(|token| token.to_string())
            .collect();

        let slug = tokens
            .into_iter()
            .filter(|token| {
                let token = token.to_lowercase();
                !FLUFF_SET.contains(&token) && !LOCATION_SET.contains(&token)
            })
            .join(" ");
        match HARDCODED_FIXES.get(&slug.to_lowercase()) {
            Some(fix) => fix.to_string(),
            None => slug.to_lowercase(),
        }
    }
}

const FLUFF: &'static [&'static str] = &[
    "ag",
    "and",
    "co",
    "corporation",
    "development",
    "east",
    "entertainment",
    "game",
    "games",
    "gmbh",
    "inc",
    "interactive",
    "international",
    "limited",
    "llc",
    "ltd",
    "media",
    "north",
    "northwest",
    "on-line",
    "online",
    "partners",
    "production",
    "productions",
    "publishing",
    "software",
    "softworks",
    "studio",
    "studios",
    "technologies",
    "the",
    "victor",
    "west",
];

const LOCATION: &'static [&'static str] = &[
    "albany",
    "asia-pacific",
    "asia",
    "austin",
    "australia",
    "baltimore",
    "birmingham",
    "boston",
    "bucharest",
    "budapest",
    "canada",
    "casablanca",
    "chicago",
    "china",
    "czech",
    "deutschland",
    "edmonton",
    "europe",
    "france",
    "frankfurt",
    "hawaii",
    "italia",
    "japan",
    "kiev",
    "london",
    "los angeles",
    "manchester",
    "marin",
    "milan",
    "monpellier",
    "montpellier",
    "montreal",
    "montréal",
    "nordic",
    "paris",
    "poland",
    "quebec",
    "québec",
    "san diego",
    "shanghai",
    "sofia",
    "southam",
    "teesside",
    "tokyo",
    "toronto",
    "uk",
    "usa",
    "vancouver",
];

static HARDCODED_FIXES: phf::Map<&'static str, &'static str> = phf_map! {
    "ea" => "electronic arts",
    "ea digital illusions ce" => "electronic arts",
    "ea sports" => "electronic arts",
    "kalypsomediagroup" => "kalypso",
    "lucasfilm" => "lucasarts",
    "lucas arts" => "lucasarts",
    "cd project red" => "cd project",
    "wb" => "warner bros",
    "3d realms" => "apogee",
};
