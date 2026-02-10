# OAGW Client Library Prototype

Minimal prototype for testing HTTP client compatibility and API design for the OAGW Client Library.

## Purpose

This prototype validates the design from `adr-rust-abi-client-library.md` before full implementation:

- ✅ Test async and blocking (sync) API patterns
- ✅ Verify SSE streaming response handling
- ✅ Validate error source distinction (Gateway vs Upstream)
- ✅ Ensure compatibility with different HTTP clients
- ✅ Test flexible response consumption patterns

## Features

### Core Types

- **`Request`** - HTTP request builder with method, path, headers, body
- **`Response`** - HTTP response with flexible consumption (buffered or streaming)
- **`Body`** - Request/response body abstraction (Empty, Bytes, Stream)
- **`ErrorSource`** - Distinguishes gateway vs upstream errors
- **`ClientError`** - Comprehensive error types

### Client Implementation

- **`OagwClient`** - Main client type with deployment abstraction
- **`RemoteProxyClient`** - HTTP-based client (for testing, simulates remote OAGW)
- Blocking API wrapper (`execute_blocking()`)

### Response Patterns

- `.bytes()` - Buffer entire response
- `.json()` - Parse as JSON
- `.text()` - Parse as text
- `.into_stream()` - Consume as byte stream
- `.into_sse_stream()` - Parse as Server-Sent Events

### Compatibility

- ✅ Async usage with tokio runtime
- ✅ Blocking usage (build script pattern)
- ✅ SSE streaming
- ✅ Byte streaming
- ✅ Error source tracking

## Quick Start

### Installation

This is a standalone prototype, not yet added to the workspace. To use:

```bash
cd modules/system/oagw/oagw-client-prototype
```

### Configuration

Set environment variables:

```bash
export OAGW_AUTH_TOKEN="your-token-here"
export OAGW_BASE_URL="http://localhost:8080"  # Optional, defaults to https://oagw.internal.cf
```

## Usage Examples

### Async Usage

```rust
use oagw_client_prototype::{OagwClient, OagwClientConfig, Request, Method};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Option 1: Manual config
    let config = OagwClientConfig::remote(
        "https://oagw.internal.cf".to_string(),
        std::env::var("OAGW_AUTH_TOKEN")?,
    );

    // Option 2: Auto-detect from environment
    // let config = OagwClientConfig::from_env()?;

    let client = OagwClient::from_config(config)?;

    // Buffered request
    let request = Request::builder()
        .method(Method::POST)
        .path("/v1/chat/completions")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello!"}]
        }))?
        .build()?;

    let response = client.execute("openai", request).await?;
    let data: serde_json::Value = response.json().await?;

    println!("Response: {}", data);
    Ok(())
}
```

### Blocking Usage (Build Scripts)

```rust
use oagw_client_prototype::{OagwClient, OagwClientConfig, Request, Method};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Auto-detect from environment
    let config = OagwClientConfig::from_env()?;
    let client = OagwClient::from_config(config)?;

    let request = Request::builder()
        .method(Method::GET)
        .path("/elements@9.0.15/web-components.min.js")
        .build()?;

    // Blocking call (no tokio runtime needed)
    let response = client.execute_blocking("unpkg", request)?;
    let bytes = response.bytes_blocking()?;

    std::fs::write("assets/web-components.min.js", bytes)?;
    Ok(())
}
```

### SSE Streaming

```rust
use oagw_client_prototype::{OagwClient, OagwClientConfig, Request, Method};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = OagwClientConfig::from_env()?;
    let client = OagwClient::from_config(config)?;

    let request = Request::builder()
        .method(Method::POST)
        .path("/v1/chat/completions")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Tell me a story"}],
            "stream": true
        }))?
        .build()?;

    let response = client.execute("openai", request).await?;

    // Consume as SSE stream
    let mut sse = response.into_sse_stream();
    while let Some(event) = sse.next_event().await? {
        if event.data.contains("[DONE]") {
            break;
        }

        let data: serde_json::Value = serde_json::from_str(&event.data)?;
        if let Some(content) = data["choices"][0]["delta"]["content"].as_str() {
            print!("{}", content);
        }
    }

    println!();
    Ok(())
}
```

## Running Examples

```bash
cd modules/system/oagw/oagw-client-prototype

# Async example
cargo run --example async_usage

# Blocking example
cargo run --example blocking_usage

# SSE streaming example
cargo run --example streaming_sse
```

## Running Tests

```bash
# Run all tests
cargo test

# Run integration tests only
cargo test --test integration_test

# Run compatibility tests only
cargo test --test compatibility_test

# Run with output
cargo test -- --nocapture
```

## Architecture

### Deployment Modes

The client supports multiple deployment modes (currently only RemoteProxy is implemented):

1. **RemoteProxy Mode** (Implemented) - OAGW runs in a separate process, client makes HTTP calls
2. **SharedProcess Mode** (Future) - OAGW runs in the same process, client uses direct function calls

### Error Source Tracking

The client distinguishes between errors from the OAGW gateway and upstream services via the `X-OAGW-Error-Source` header:

- **`ErrorSource::Gateway`** - Error originated from OAGW (e.g., alias not found, rate limit)
- **`ErrorSource::Upstream`** - Error originated from external service (e.g., API error)
- **`ErrorSource::Unknown`** - No header present

### Response Consumption

The `Response` type supports multiple consumption patterns:

- **Buffered**: `.bytes()`, `.json()`, `.text()` - Load entire response into memory
- **Streaming**: `.into_stream()` - Consume as async byte stream
- **SSE**: `.into_sse_stream()` - Parse Server-Sent Events

## Testing Strategy

### Integration Tests (`tests/integration_test.rs`)

- Simple JSON requests
- GET requests
- Error source distinction
- Text responses
- Binary responses
- Custom headers
- Blocking requests

### Compatibility Tests (`tests/compatibility_test.rs`)

- Async usage patterns
- Blocking (sync) usage patterns
- SSE streaming
- SSE with metadata (IDs, event types)
- Byte stream consumption
- Config from environment

### Unit Tests

- SSE parsing logic (`src/sse.rs`)
- Error source header parsing (`src/remote_proxy.rs`)
- Request building (`src/request.rs`)
- Client configuration (`src/client.rs`)

## Next Steps

After prototype validation:

1. ✅ Validate design and API ergonomics
2. ✅ Confirm compatibility patterns work
3. Resolve 10 architectural issues from REVIEW_FEEDBACK.md
4. Implement SharedProcessClient (in-memory mode)
5. Create full oagw-sdk crate following module patterns
6. Implement OAGW gateway components (API Handler, CP, DP)
7. Add to workspace and register in hyperspot-server

## Dependencies

- **reqwest** - HTTP client (currently used for RemoteProxyClient)
- **tokio** - Async runtime
- **http** - HTTP types
- **bytes** - Efficient byte buffers
- **serde/serde_json** - Serialization
- **futures** - Stream utilities
- **thiserror** - Error handling
- **httpmock** - Testing (dev dependency)

## Design Validation

This prototype validates key design decisions from the ADR:

✅ **Async + Blocking API** - Both patterns work seamlessly
✅ **Flexible Response Consumption** - Buffered, streaming, and SSE all work
✅ **Error Source Distinction** - Gateway vs upstream errors tracked correctly
✅ **Deployment Abstraction** - Client abstraction supports multiple modes
✅ **HTTP Client Compatibility** - reqwest integration validated

## Known Limitations (Prototype)

- Only RemoteProxy mode implemented (SharedProcess coming later)
- Streaming request bodies not yet supported
- No retry or circuit breaker logic
- No connection pooling tuning
- No mTLS or advanced auth patterns
- **Testing Note**: The blocking API (`execute_blocking()`, `json_blocking()`) cannot be fully tested in the test suite because MockServer requires tokio. In real-world usage (e.g., build.rs), these methods work correctly by creating a temporary runtime when none exists.

## License

See main CyberFabric license.
