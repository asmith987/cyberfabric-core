use bytes::Bytes;
use futures::StreamExt;
use http::{HeaderMap, StatusCode};
use serde::de::DeserializeOwned;

use crate::body::BoxStream;
use crate::error::{ClientError, ErrorSource};
use crate::sse::SseEventStream;

/// HTTP response with flexible consumption patterns
pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: ResponseBody,
    error_source: ErrorSource,
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("error_source", &self.error_source)
            .finish()
    }
}

enum ResponseBody {
    Buffered(Bytes),
    Streaming(BoxStream<Result<Bytes, ClientError>>),
}

impl std::fmt::Debug for ResponseBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseBody::Buffered(bytes) => {
                f.debug_tuple("ResponseBody::Buffered").field(&bytes.len()).finish()
            }
            ResponseBody::Streaming(_) => write!(f, "ResponseBody::Streaming(..)"),
        }
    }
}

impl Response {
    /// Create a new response from components
    pub fn new(
        status: StatusCode,
        headers: HeaderMap,
        stream: BoxStream<Result<Bytes, ClientError>>,
        error_source: ErrorSource,
    ) -> Self {
        Self {
            status,
            headers,
            body: ResponseBody::Streaming(stream),
            error_source,
        }
    }

    /// Create a response from buffered bytes
    pub fn from_bytes(
        status: StatusCode,
        headers: HeaderMap,
        bytes: Bytes,
        error_source: ErrorSource,
    ) -> Self {
        Self {
            status,
            headers,
            body: ResponseBody::Buffered(bytes),
            error_source,
        }
    }

    /// Get the HTTP status code
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Get the response headers
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get the error source
    pub fn error_source(&self) -> ErrorSource {
        self.error_source
    }

    /// Consume the response and return the entire body as bytes
    pub async fn bytes(self) -> Result<Bytes, ClientError> {
        match self.body {
            ResponseBody::Buffered(bytes) => Ok(bytes),
            ResponseBody::Streaming(mut stream) => {
                let mut buf = Vec::new();
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    buf.extend_from_slice(&chunk);
                }
                Ok(Bytes::from(buf))
            }
        }
    }

    /// Blocking version of bytes() for sync contexts
    pub fn bytes_blocking(self) -> Result<Bytes, ClientError> {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle.block_on(self.bytes()),
            Err(_) => tokio::runtime::Runtime::new()?.block_on(self.bytes()),
        }
    }

    /// Consume the response and deserialize as JSON
    pub async fn json<T: DeserializeOwned>(self) -> Result<T, ClientError> {
        let bytes = self.bytes().await?;
        let value = serde_json::from_slice(&bytes)?;
        Ok(value)
    }

    /// Blocking version of json() for sync contexts
    pub fn json_blocking<T: DeserializeOwned>(self) -> Result<T, ClientError> {
        let bytes = self.bytes_blocking()?;
        let value = serde_json::from_slice(&bytes)?;
        Ok(value)
    }

    /// Consume the response and return the body as a string
    pub async fn text(self) -> Result<String, ClientError> {
        let bytes = self.bytes().await?;
        String::from_utf8(bytes.to_vec())
            .map_err(|e| ClientError::InvalidResponse(format!("Invalid UTF-8: {}", e)))
    }

    /// Blocking version of text() for sync contexts
    pub fn text_blocking(self) -> Result<String, ClientError> {
        let bytes = self.bytes_blocking()?;
        String::from_utf8(bytes.to_vec())
            .map_err(|e| ClientError::InvalidResponse(format!("Invalid UTF-8: {}", e)))
    }

    /// Convert the response into a byte stream for streaming consumption
    pub fn into_stream(self) -> BoxStream<Result<Bytes, ClientError>> {
        match self.body {
            ResponseBody::Buffered(bytes) => {
                Box::pin(futures::stream::once(async move { Ok(bytes) }))
            }
            ResponseBody::Streaming(stream) => stream,
        }
    }

    /// Convert the response into a Server-Sent Events stream
    pub fn into_sse_stream(self) -> SseEventStream {
        let stream = self.into_stream();
        SseEventStream::new(stream)
    }
}
