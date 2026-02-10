use std::io;
use thiserror::Error;
use bytes::Bytes;
use http::StatusCode;

/// Distinguishes whether an error originated from the OAGW gateway or the upstream service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSource {
    /// Error originated from the OAGW gateway itself
    Gateway,
    /// Error originated from the upstream external service
    Upstream,
    /// Error source unknown (no X-OAGW-Error-Source header)
    Unknown,
}

/// Comprehensive error types for OAGW client operations
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Request build error: {0}")]
    BuildError(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("HTTP error: status={status}")]
    Http { status: StatusCode, body: Bytes },

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl From<serde_json::Error> for ClientError {
    fn from(err: serde_json::Error) -> Self {
        ClientError::Serialization(err.to_string())
    }
}
