mod firestore;
mod gog;
mod gog_token;
mod igdb;
mod metacritic;
mod steam;
mod steam_scrape;
mod wikipedia_scrape;

pub use firestore::FirestoreApi;
pub use gog::GogApi;
pub use gog_token::GogToken;
pub use igdb::*;
pub use metacritic::{MetacriticApi, MetacriticData};
pub use steam::SteamApi;
pub use steam_scrape::{SteamScrape, SteamScrapeData};
pub use wikipedia_scrape::{WikipediaScrape, WikipediaScrapeData};
