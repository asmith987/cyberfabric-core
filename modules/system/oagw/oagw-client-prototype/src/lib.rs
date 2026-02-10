//! OAGW Client Library Prototype
//!
//! Minimal prototype for testing HTTP client compatibility and API design
//! for the Outbound API Gateway (OAGW) client library.
//!
//! This prototype validates the design from `adr-rust-abi-client-library.md` before
//! full implementation:
//!
//! - ✅ Test async and blocking (sync) API patterns
//! - ✅ Verify SSE streaming response handling
//! - ✅ Validate error source distinction (Gateway vs Upstream)
//! - ✅ Ensure compatibility with different HTTP clients
//! - ✅ Test flexible response consumption patterns
//!
//! # Examples
//!
//! ## Async Usage
//!
//! ```no_run
//! use oagw_client_prototype::{OagwClient, OagwClientConfig, Request};
//! use http::Method;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = OagwClientConfig::remote(
//!     "http://localhost:8080".to_string(),
//!     "test-token".to_string(),
//! );
//! let client = OagwClient::from_config(config)?;
//!
//! let request = Request::builder()
//!     .method(Method::POST)
//!     .path("/v1/chat/completions")
//!     .json(&json!({"model": "gpt-4"}))?
//!     .build()?;
//!
//! let response = client.execute("openai", request).await?;
//! let data: serde_json::Value = response.json().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Blocking Usage (Build Scripts)
//!
//! ```no_run
//! use oagw_client_prototype::{OagwClient, OagwClientConfig, Request};
//! use http::Method;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = OagwClientConfig::from_env()?;
//! let client = OagwClient::from_config(config)?;
//!
//! let request = Request::builder()
//!     .method(Method::GET)
//!     .path("/package.json")
//!     .build()?;
//!
//! let response = client.execute_blocking("unpkg", request)?;
//! let bytes = response.bytes_blocking()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## SSE Streaming
//!
//! ```no_run
//! use oagw_client_prototype::{OagwClient, OagwClientConfig, Request};
//! use http::Method;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = OagwClientConfig::from_env()?;
//! let client = OagwClient::from_config(config)?;
//!
//! let request = Request::builder()
//!     .method(Method::POST)
//!     .path("/v1/chat/completions")
//!     .json(&json!({"model": "gpt-4", "stream": true}))?
//!     .build()?;
//!
//! let response = client.execute("openai", request).await?;
//! let mut sse = response.into_sse_stream();
//!
//! while let Some(event) = sse.next_event().await? {
//!     println!("Event: {}", event.data);
//! }
//! # Ok(())
//! # }
//! ```

mod body;
mod client;
mod error;
mod remote_proxy;
mod request;
mod response;
mod sse;

// Re-export public API
pub use body::Body;
pub use client::{ClientMode, OagwClient, OagwClientConfig};
pub use error::{ClientError, ErrorSource};
pub use request::{Request, RequestBuilder};
pub use response::Response;
pub use sse::{SseEvent, SseEventStream};

// Re-export commonly used types from dependencies
pub use http::{Method, StatusCode};
