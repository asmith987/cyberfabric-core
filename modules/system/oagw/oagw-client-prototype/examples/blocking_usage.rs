//! Blocking (sync) usage example for OAGW client library
//!
//! This example demonstrates blocking API usage patterns, suitable for:
//! - Build scripts (build.rs)
//! - Sync contexts where async runtime is not available
//! - Simple scripts that don't need async
//!
//! To run this example:
//! ```bash
//! export OAGW_AUTH_TOKEN="your-token-here"
//! export OAGW_BASE_URL="http://localhost:8080"  # Optional
//! cargo run --example blocking_usage
//! ```

use http::Method;
use oagw_client_prototype::{OagwClient, OagwClientConfig, Request};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Auto-detect configuration from environment
    let config = OagwClientConfig::from_env()?;
    let client = OagwClient::from_config(config)?;

    println!("=== Example 1: Blocking JSON Request ===\n");

    let request = Request::builder()
        .method(Method::GET)
        .path("/package.json")
        .build()?;

    // Blocking call - no tokio runtime needed!
    let response = client.execute_blocking("unpkg", request)?;

    println!("Status: {}", response.status());
    println!("Error Source: {:?}", response.error_source());

    // Blocking JSON parsing
    let data: serde_json::Value = response.json_blocking()?;
    println!("Package name: {}\n", data["name"]);

    println!("=== Example 2: Download Binary File ===\n");

    let request = Request::builder()
        .method(Method::GET)
        .path("/react@18.2.0/umd/react.production.min.js")
        .build()?;

    let response = client.execute_blocking("unpkg", request)?;

    // Blocking bytes retrieval
    let bytes = response.bytes_blocking()?;
    println!("Downloaded {} bytes\n", bytes.len());

    // Could write to file:
    // std::fs::write("assets/react.min.js", bytes)?;

    println!("=== Example 3: Text Response ===\n");

    let request = Request::builder()
        .method(Method::GET)
        .path("/")
        .build()?;

    let response = client.execute_blocking("example", request)?;
    let text = response.text_blocking()?;

    println!("Text length: {} chars\n", text.len());

    println!("=== Build Script Pattern ===\n");
    println!("This example can be used directly in build.rs:");
    println!("
    // build.rs
    use oagw_client_prototype::{{OagwClient, OagwClientConfig, Request}};
    use http::Method;

    fn main() -> Result<(), Box<dyn std::error::Error>> {{
        let config = OagwClientConfig::from_env()?;
        let client = OagwClient::from_config(config)?;

        let request = Request::builder()
            .method(Method::GET)
            .path(\"/web-components.min.js\")
            .build()?;

        let response = client.execute_blocking(\"unpkg\", request)?;
        let bytes = response.bytes_blocking()?;

        std::fs::write(\"assets/web-components.min.js\", bytes)?;
        println!(\"cargo:rerun-if-changed=build.rs\");

        Ok(())
    }}
    ");

    println!("\n=== All examples completed successfully! ===");

    Ok(())
}
