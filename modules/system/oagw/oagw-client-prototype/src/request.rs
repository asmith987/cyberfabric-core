use http::{HeaderMap, HeaderName, HeaderValue, Method};
use serde::Serialize;
use std::time::Duration;

use crate::body::Body;
use crate::error::ClientError;

/// HTTP request with method, path, headers, and body
#[derive(Debug)]
pub struct Request {
    method: Method,
    path: String,
    headers: HeaderMap,
    body: Body,
    timeout: Option<Duration>,
}

impl Request {
    /// Create a new request builder
    pub fn builder() -> RequestBuilder {
        RequestBuilder::default()
    }

    /// Get the HTTP method
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Get the request path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the request headers
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get a mutable reference to headers
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Get the request body
    pub fn body(&self) -> &Body {
        &self.body
    }

    /// Take the request body
    pub fn into_body(self) -> Body {
        self.body
    }

    /// Get the timeout duration
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
}

/// Builder for constructing HTTP requests with a fluent API
#[derive(Debug, Default)]
pub struct RequestBuilder {
    method: Option<Method>,
    path: Option<String>,
    headers: HeaderMap,
    body: Body,
    timeout: Option<Duration>,
}

impl RequestBuilder {
    /// Set the HTTP method
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    /// Set the request path
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Add a header
    pub fn header<K, V>(mut self, key: K, value: V) -> Result<Self, ClientError>
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
        K::Error: std::fmt::Display,
        V::Error: std::fmt::Display,
    {
        let key = key
            .try_into()
            .map_err(|e| ClientError::BuildError(format!("Invalid header name: {}", e)))?;
        let value = value
            .try_into()
            .map_err(|e| ClientError::BuildError(format!("Invalid header value: {}", e)))?;
        self.headers.insert(key, value);
        Ok(self)
    }

    /// Set the body to a JSON-serialized value and add Content-Type header
    pub fn json<T: Serialize>(mut self, value: &T) -> Result<Self, ClientError> {
        self.body = Body::from_json(value)?;
        self.headers.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        Ok(self)
    }

    /// Set the request body
    pub fn body<B: Into<Body>>(mut self, body: B) -> Self {
        self.body = body.into();
        self
    }

    /// Set request timeout
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Build the request
    pub fn build(self) -> Result<Request, ClientError> {
        let method = self.method.unwrap_or(Method::GET);
        let path = self
            .path
            .ok_or_else(|| ClientError::BuildError("Request path is required".into()))?;

        Ok(Request {
            method,
            path,
            headers: self.headers,
            body: self.body,
            timeout: self.timeout,
        })
    }
}
