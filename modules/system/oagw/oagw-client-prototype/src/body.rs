use bytes::Bytes;
use futures::stream::Stream;
use serde::Serialize;
use std::io;
use std::pin::Pin;

use crate::error::ClientError;

pub type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send + 'static>>;

/// HTTP request/response body abstraction
pub enum Body {
    /// Empty body
    Empty,
    /// Buffered bytes
    Bytes(Bytes),
    /// Streaming body
    Stream(BoxStream<Result<Bytes, io::Error>>),
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Body::Empty => write!(f, "Body::Empty"),
            Body::Bytes(bytes) => f.debug_tuple("Body::Bytes").field(&bytes.len()).finish(),
            Body::Stream(_) => write!(f, "Body::Stream(..)"),
        }
    }
}

impl Body {
    /// Create an empty body
    pub fn empty() -> Self {
        Body::Empty
    }

    /// Create a body from bytes
    pub fn from_bytes(bytes: impl Into<Bytes>) -> Self {
        Body::Bytes(bytes.into())
    }

    /// Create a body from a JSON-serializable value
    pub fn from_json<T: Serialize>(value: &T) -> Result<Self, ClientError> {
        let json = serde_json::to_vec(value)?;
        Ok(Body::Bytes(Bytes::from(json)))
    }

    /// Check if body is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, Body::Empty)
    }
}

impl Default for Body {
    fn default() -> Self {
        Body::Empty
    }
}

// Convenient From trait implementations
impl From<()> for Body {
    fn from(_: ()) -> Self {
        Body::Empty
    }
}

impl From<String> for Body {
    fn from(s: String) -> Self {
        Body::Bytes(Bytes::from(s))
    }
}

impl From<&str> for Body {
    fn from(s: &str) -> Self {
        Body::Bytes(Bytes::from(s.to_string()))
    }
}

impl From<Vec<u8>> for Body {
    fn from(v: Vec<u8>) -> Self {
        Body::Bytes(Bytes::from(v))
    }
}

impl From<Bytes> for Body {
    fn from(b: Bytes) -> Self {
        Body::Bytes(b)
    }
}
