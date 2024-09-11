mod common;
mod firestore;
mod gog;
mod igdb;
mod metacritic;
mod steam;
mod wikipedia;

pub use common::CompanyNormalizer;
pub use firestore::FirestoreApi;
pub use gog::*;
pub use igdb::*;
pub use metacritic::{MetacriticApi, MetacriticData};
pub use steam::*;
pub use wikipedia::Wikipedia;
