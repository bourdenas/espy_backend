mod firestore;
mod gog;
mod gog_token;
mod igdb;
mod metacritic;
mod steam;
mod wikipedia_scrape;

pub use firestore::FirestoreApi;
pub use gog::GogApi;
pub use gog_token::GogToken;
pub use igdb::*;
pub use metacritic::{MetacriticApi, MetacriticData};
pub use steam::*;
pub use wikipedia_scrape::{WikipediaScrape, WikipediaScrapeData};
