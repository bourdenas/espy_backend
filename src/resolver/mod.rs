mod client;
mod handlers;
mod igdb;
pub mod models;
pub mod routes;

pub use client::ResolveApi;
pub use igdb::{endpoints, filtering, IgdbBatchApi, IgdbConnection, IgdbLookup};
