mod batch;
mod connection;
pub mod endpoints;
mod ranking;
mod request;
mod resolve;
mod search;
mod service;

pub use batch::IgdbBatchApi;
pub use connection::IgdbConnection;
pub use search::IgdbSearch;
pub use service::IgdbApi;
