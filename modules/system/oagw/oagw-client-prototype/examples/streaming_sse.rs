//! Server-Sent Events (SSE) streaming example
//!
//! This example demonstrates SSE streaming patterns for:
//! - OpenAI streaming chat completions
//! - Real-time event streams
//! - Progressive data loading
//!
//! To run this example:
//! ```bash
//! export OAGW_AUTH_TOKEN="your-token-here"
//! export OAGW_BASE_URL="http://localhost:8080"  # Optional
//! cargo run --example streaming_sse
//! ```

use http::Method;
use oagw_client_prototype::{OagwClient, OagwClientConfig, Request};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = OagwClientConfig::from_env()?;
    let client = OagwClient::from_config(config)?;

    println!("=== Example 1: OpenAI Streaming Chat ===\n");

    let request = Request::builder()
        .method(Method::POST)
        .path("/v1/chat/completions")
        .json(&json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Tell me a short story about a robot learning to code."}
            ],
            "stream": true,
            "max_tokens": 200
        }))?
        .build()?;

    let response = client.execute("openai", request).await?;

    println!("Status: {}", response.status());
    println!("Starting stream...\n");

    // Consume as SSE stream
    let mut sse = response.into_sse_stream();
    let mut full_content = String::new();
    let mut event_count = 0;

    while let Some(event) = sse.next_event().await? {
        event_count += 1;

        // Check for done signal
        if event.data.contains("[DONE]") {
            break;
        }

        // Parse the event data
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.data) {
            // Extract content delta
            if let Some(content) = data["choices"][0]["delta"]["content"].as_str() {
                print!("{}", content);
                full_content.push_str(content);
                std::io::Write::flush(&mut std::io::stdout())?;
            }
        }
    }

    println!("\n\n=== Stream Complete ===");
    println!("Events received: {}", event_count);
    println!("Total content length: {} chars\n", full_content.len());

    println!("=== Example 2: Custom SSE Events ===\n");

    // Example with custom event types and IDs
    let request = Request::builder()
        .method(Method::GET)
        .path("/events")
        .build()?;

    let response = client.execute("example", request).await?;
    let mut sse = response.into_sse_stream();

    println!("Listening for events...\n");

    let mut event_num = 0;
    while let Some(event) = sse.next_event().await? {
        event_num += 1;

        println!("Event #{}:", event_num);
        if let Some(id) = &event.id {
            println!("  ID: {}", id);
        }
        if let Some(event_type) = &event.event {
            println!("  Type: {}", event_type);
        }
        println!("  Data: {}", event.data);
        if let Some(retry) = event.retry {
            println!("  Retry: {}ms", retry);
        }
        println!();

        // Stop after 10 events for demo
        if event_num >= 10 {
            break;
        }
    }

    println!("=== Example 3: Byte Stream (Raw) ===\n");

    // For non-SSE streaming, use into_stream()
    let request = Request::builder()
        .method(Method::GET)
        .path("/large-file.json")
        .build()?;

    let response = client.execute("example", request).await?;
    let mut stream = response.into_stream();

    use futures::StreamExt;

    let mut total_bytes = 0;
    let mut chunk_count = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        total_bytes += chunk.len();
        chunk_count += 1;

        if chunk_count % 10 == 0 {
            println!("Received {} chunks, {} bytes so far...", chunk_count, total_bytes);
        }
    }

    println!("\nTotal: {} chunks, {} bytes\n", chunk_count, total_bytes);

    println!("=== All examples completed successfully! ===");

    Ok(())
}
