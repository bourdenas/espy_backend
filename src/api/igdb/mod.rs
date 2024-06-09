mod backend;
mod batch;
mod connection;
mod docs;
mod ranking;
mod resolve;
mod search;
mod service;
mod webhooks;

pub use batch::IgdbBatchApi;
use connection::IgdbConnection;
pub use docs::{IgdbExternalGame, IgdbGame, IgdbGameDiff, IgdbGenre};
pub use search::IgdbSearch;
pub use service::IgdbApi;
pub use webhooks::IgdbWebhooksApi;
