//! Async usage example for OAGW client library
//!
//! This example demonstrates typical async usage patterns with the OAGW client.
//!
//! To run this example:
//! ```bash
//! export OAGW_AUTH_TOKEN="your-token-here"
//! export OAGW_BASE_URL="http://localhost:8080"  # Optional, defaults to https://oagw.internal.cf
//! cargo run --example async_usage
//! ```

use http::Method;
use oagw_client_prototype::{OagwClient, OagwClientConfig, Request};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Option 1: Manual configuration
    let config = OagwClientConfig::remote(
        std::env::var("OAGW_BASE_URL")
            .unwrap_or_else(|_| "https://oagw.internal.cf".to_string()),
        std::env::var("OAGW_AUTH_TOKEN")
            .expect("OAGW_AUTH_TOKEN environment variable must be set"),
    );

    // Option 2: Auto-detect from environment
    // let config = OagwClientConfig::from_env()?;

    let client = OagwClient::from_config(config)?;

    println!("=== Example 1: Simple JSON Request ===\n");

    // Buffered JSON request
    let request = Request::builder()
        .method(Method::POST)
        .path("/v1/chat/completions")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Say 'Hello, OAGW!'"}],
            "max_tokens": 50
        }))?
        .build()?;

    let response = client.execute("openai", request).await?;

    println!("Status: {}", response.status());
    println!("Error Source: {:?}", response.error_source());

    let data: serde_json::Value = response.json().await?;
    println!("Response: {}\n", serde_json::to_string_pretty(&data)?);

    println!("=== Example 2: GET Request ===\n");

    // Simple GET request
    let request = Request::builder()
        .method(Method::GET)
        .path("/users/octocat")
        .build()?;

    let response = client.execute("github", request).await?;

    println!("Status: {}", response.status());

    let text = response.text().await?;
    println!("Response body: {}\n", text);

    println!("=== Example 3: Custom Headers ===\n");

    // Request with custom headers
    let request = Request::builder()
        .method(Method::GET)
        .path("/v1/models")
        .header("X-Request-ID", "example-123")?
        .build()?;

    let response = client.execute("openai", request).await?;

    println!("Status: {}", response.status());
    println!("Headers: {:?}\n", response.headers());

    let data: serde_json::Value = response.json().await?;
    println!("Available models: {}\n", data["data"].as_array().map(|v| v.len()).unwrap_or(0));

    println!("=== All examples completed successfully! ===");

    Ok(())
}
