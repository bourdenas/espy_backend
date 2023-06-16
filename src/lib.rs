#![recursion_limit = "256"]

pub mod api;
pub mod documents;
pub mod games;
pub mod http;
pub mod library;
pub mod traits;
pub mod util;
pub mod webhooks;

mod status;
pub use status::Status;

mod tracing;
pub use crate::tracing::Tracing;
