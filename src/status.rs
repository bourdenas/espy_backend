use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{error::Error, fmt};
use valuable::Valuable;

#[derive(Serialize, Deserialize, Valuable, Default, Clone, Debug)]
pub enum Status {
    #[default]
    Ok,

    Internal(String),
    InvalidArgument(String),
    NotFound(String),
    RequestError(String),
}

impl Status {
    pub fn new(msg: &str, err: impl Error) -> Self {
        Status::Internal(format!("{msg}: '{err}'"))
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Status::Internal(msg.into())
    }

    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Status::InvalidArgument(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Status::NotFound(msg.into())
    }

    pub fn request_error(msg: impl Into<String>) -> Self {
        Status::RequestError(msg.into())
    }
}

impl From<std::io::Error> for Status {
    fn from(err: std::io::Error) -> Self {
        Self::new("IO error", err)
    }
}

impl From<serde_json::Error> for Status {
    fn from(err: serde_json::Error) -> Self {
        Self::new("serde error", err)
    }
}

impl From<reqwest::Error> for Status {
    fn from(err: reqwest::Error) -> Self {
        Self::request_error(err.to_string())
    }
}

use firestore::errors::FirestoreError;
impl From<FirestoreError> for Status {
    fn from(err: FirestoreError) -> Self {
        match err {
            FirestoreError::DataNotFoundError(err) => Self::not_found(err.to_string()),
            FirestoreError::InvalidParametersError(err) => Self::invalid_argument(err.to_string()),
            FirestoreError::SerializeError(err) => Self::internal(format!(
                "Failed to serialize document with error '{}'",
                err.message
            )),
            FirestoreError::DeserializeError(err) => Self::internal(format!(
                "Failed to parse document with error '{}'",
                err.message
            )),
            err => Self::new("Firestore error", err),
        }
    }
}

impl Error for Status {}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Ok => write!(f, "Ok"),
            Status::Internal(msg) => write!(f, "Interal error: {msg}"),
            Status::InvalidArgument(msg) => write!(f, "Invalid argument error: {msg}"),
            Status::NotFound(msg) => write!(f, "Not found error: {msg}"),
            Status::RequestError(msg) => write!(f, "Request error: {msg}"),
        }
    }
}
