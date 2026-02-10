use futures::TryStreamExt;
use http::HeaderMap;
use std::time::Duration;

use crate::body::Body;
use crate::error::{ClientError, ErrorSource};
use crate::request::Request;
use crate::response::Response;

/// HTTP-based client that routes requests through OAGW proxy endpoints
pub struct RemoteProxyClient {
    oagw_base_url: String,
    http_client: reqwest::Client,
    auth_token: String,
}

impl RemoteProxyClient {
    /// Create a new remote proxy client
    pub fn new(
        base_url: String,
        auth_token: String,
        timeout: Duration,
    ) -> Result<Self, ClientError> {
        let http_client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ClientError::BuildError(e.to_string()))?;

        Ok(Self {
            oagw_base_url: base_url,
            http_client,
            auth_token,
        })
    }

    /// Execute an HTTP request through the OAGW proxy
    pub async fn execute(&self, alias: &str, request: Request) -> Result<Response, ClientError> {
        // Build URL: {base_url}/api/oagw/v1/proxy/{alias}{path}
        let url = format!(
            "{}/api/oagw/v1/proxy/{}{}",
            self.oagw_base_url,
            alias,
            request.path()
        );

        // Build reqwest request
        let mut req_builder = self
            .http_client
            .request(request.method().clone(), &url)
            .header("Authorization", format!("Bearer {}", self.auth_token));

        // Forward headers from the request
        for (name, value) in request.headers() {
            req_builder = req_builder.header(name, value);
        }

        // Apply request-specific timeout if set (before consuming request)
        let req_timeout = request.timeout();
        if let Some(timeout) = req_timeout {
            req_builder = req_builder.timeout(timeout);
        }

        // Set body based on request body type
        req_builder = match request.into_body() {
            Body::Empty => req_builder,
            Body::Bytes(bytes) => req_builder.body(bytes),
            Body::Stream(_) => {
                return Err(ClientError::BuildError(
                    "Streaming request bodies not yet supported in RemoteProxyClient".into(),
                ))
            }
        };

        // Execute the request
        let resp = req_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                ClientError::Timeout(e.to_string())
            } else if e.is_connect() {
                ClientError::Connection(e.to_string())
            } else {
                ClientError::Reqwest(e)
            }
        })?;

        let status = resp.status();
        let headers = resp.headers().clone();

        // Parse X-OAGW-Error-Source header
        let error_source = parse_error_source_header(&headers);

        // Convert response to streaming
        let stream = resp.bytes_stream().map_err(|e| {
            ClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        });

        Ok(Response::new(
            status,
            headers,
            Box::pin(stream),
            error_source,
        ))
    }
}

/// Parse the X-OAGW-Error-Source header to determine error origin
fn parse_error_source_header(headers: &HeaderMap) -> ErrorSource {
    const ERROR_SOURCE_HEADER: &str = "x-oagw-error-source";

    headers
        .get(ERROR_SOURCE_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| match s.to_lowercase().as_str() {
            "gateway" => ErrorSource::Gateway,
            "upstream" => ErrorSource::Upstream,
            _ => ErrorSource::Unknown,
        })
        .unwrap_or(ErrorSource::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::header::HeaderValue;

    #[test]
    fn test_parse_error_source_gateway() {
        let mut headers = HeaderMap::new();
        headers.insert("x-oagw-error-source", HeaderValue::from_static("gateway"));
        assert_eq!(parse_error_source_header(&headers), ErrorSource::Gateway);
    }

    #[test]
    fn test_parse_error_source_upstream() {
        let mut headers = HeaderMap::new();
        headers.insert("x-oagw-error-source", HeaderValue::from_static("upstream"));
        assert_eq!(parse_error_source_header(&headers), ErrorSource::Upstream);
    }

    #[test]
    fn test_parse_error_source_unknown() {
        let headers = HeaderMap::new();
        assert_eq!(parse_error_source_header(&headers), ErrorSource::Unknown);
    }

    #[test]
    fn test_parse_error_source_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert("x-oagw-error-source", HeaderValue::from_static("invalid"));
        assert_eq!(parse_error_source_header(&headers), ErrorSource::Unknown);
    }
}
