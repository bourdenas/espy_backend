mod batch;
mod connection;
pub mod endpoints;
mod lookups;
mod ranking;
mod request;
mod resolve;
mod search;
mod service;

pub use batch::IgdbBatchApi;
pub use connection::IgdbConnection;
pub use lookups::IgdbLookup;
pub use search::IgdbSearch;
pub use service::IgdbApi;
