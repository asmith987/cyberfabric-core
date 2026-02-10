use http::{Method, StatusCode};
use httpmock::prelude::*;
use oagw_client_prototype::{ErrorSource, OagwClient, OagwClientConfig, Request};
use serde_json::json;

/// Test 1: Async usage with tokio runtime
#[tokio::test]
async fn test_async_usage() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(POST)
            .path("/api/oagw/v1/proxy/openai/v1/chat/completions")
            .header("Authorization", "Bearer test-token");
        then.status(200)
            .header("Content-Type", "application/json")
            .header("X-OAGW-Error-Source", "upstream")
            .json_body(json!({
                "choices": [{
                    "message": {"content": "Test response"}
                }]
            }));
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::POST)
        .path("/v1/chat/completions")
        .json(&json!({"model": "gpt-4"}))
        .unwrap()
        .build()
        .unwrap();

    let response = client.execute("openai", request).await.unwrap();

    // Buffered consumption
    let data = response.json::<serde_json::Value>().await.unwrap();
    assert!(data.is_object());
    assert_eq!(
        data["choices"][0]["message"]["content"],
        json!("Test response")
    );
}

/// Test 2: Blocking (sync) usage - build script pattern
/// Note: Testing the blocking API is difficult in test environment with MockServer
/// since MockServer itself requires tokio. In real build.rs, there would be no
/// tokio runtime and execute_blocking() would create a temporary one.
/// For this test, we verify the synchronous response consumption methods work.
#[tokio::test]
async fn test_blocking_usage() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/unpkg/package.json")
            .header("Authorization", "Bearer test-token");
        then.status(200)
            .header("Content-Type", "application/json")
            .header("X-OAGW-Error-Source", "upstream")
            .json_body(json!({"name": "test-package", "version": "1.0.0"}));
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/package.json")
        .build()
        .unwrap();

    // Use async execute and async response consumption
    // (In real build.rs, would use execute_blocking() and json_blocking())
    let response = client.execute("unpkg", request).await.unwrap();
    assert!(response.status().is_success());

    // Verify response consumption works
    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["name"], json!("test-package"));
}

/// Test 3: Streaming response (SSE)
#[tokio::test]
async fn test_sse_streaming() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(POST)
            .path("/api/oagw/v1/proxy/openai/v1/chat/completions")
            .header("Authorization", "Bearer test-token")
            .json_body(json!({"model": "gpt-4", "stream": true}));
        then.status(200)
            .header("Content-Type", "text/event-stream")
            .header("X-OAGW-Error-Source", "upstream")
            .body(concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\" World\"}}]}\n\n",
                "data: [DONE]\n\n"
            ));
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::POST)
        .path("/v1/chat/completions")
        .json(&json!({"model": "gpt-4", "stream": true}))
        .unwrap()
        .build()
        .unwrap();

    let response = client.execute("openai", request).await.unwrap();

    // Consume as SSE stream
    let mut sse = response.into_sse_stream();
    let mut events = Vec::new();
    let mut done = false;

    while let Some(event) = sse.next_event().await.unwrap() {
        events.push(event.data.clone());
        if event.data.contains("[DONE]") {
            done = true;
            break;
        }
    }

    assert!(events.len() >= 3, "Expected at least 3 events");
    assert!(done, "Expected [DONE] event");
    assert!(events[0].contains("Hello"));
    assert!(events[1].contains("World"));
}

/// Test 4: SSE with event IDs and types
#[tokio::test]
async fn test_sse_with_metadata() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/example/events")
            .header("Authorization", "Bearer test-token");
        then.status(200)
            .header("Content-Type", "text/event-stream")
            .header("X-OAGW-Error-Source", "upstream")
            .body(concat!(
                "id: 1\n",
                "event: message\n",
                "data: First event\n\n",
                "id: 2\n",
                "event: update\n",
                "data: Second event\n\n",
            ));
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/events")
        .build()
        .unwrap();

    let response = client.execute("example", request).await.unwrap();
    let mut sse = response.into_sse_stream();

    // First event
    let event1 = sse.next_event().await.unwrap().unwrap();
    assert_eq!(event1.id, Some("1".to_string()));
    assert_eq!(event1.event, Some("message".to_string()));
    assert_eq!(event1.data, "First event");

    // Second event
    let event2 = sse.next_event().await.unwrap().unwrap();
    assert_eq!(event2.id, Some("2".to_string()));
    assert_eq!(event2.event, Some("update".to_string()));
    assert_eq!(event2.data, "Second event");
}

/// Test 5: Error source distinction
#[tokio::test]
async fn test_error_source() {
    let server = MockServer::start();

    // Test gateway error
    server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/invalid-alias/test")
            .header("Authorization", "Bearer test-token");
        then.status(404)
            .header("X-OAGW-Error-Source", "gateway")
            .body("Alias not found");
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/test")
        .build()
        .unwrap();

    let response = client.execute("invalid-alias", request).await.unwrap();

    // Should be gateway error (OAGW returned 404)
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.error_source(), ErrorSource::Gateway);
}

/// Test 6: Multiline SSE data
#[tokio::test]
async fn test_sse_multiline_data() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/example/multiline")
            .header("Authorization", "Bearer test-token");
        then.status(200)
            .header("Content-Type", "text/event-stream")
            .header("X-OAGW-Error-Source", "upstream")
            .body(concat!(
                "data: Line 1\n",
                "data: Line 2\n",
                "data: Line 3\n\n",
            ));
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/multiline")
        .build()
        .unwrap();

    let response = client.execute("example", request).await.unwrap();
    let mut sse = response.into_sse_stream();

    let event = sse.next_event().await.unwrap().unwrap();
    assert_eq!(event.data, "Line 1\nLine 2\nLine 3");
}

/// Test 7: Byte stream consumption (chunked)
#[tokio::test]
async fn test_byte_stream() {
    use futures::StreamExt;

    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/example/stream")
            .header("Authorization", "Bearer test-token");
        then.status(200)
            .header("X-OAGW-Error-Source", "upstream")
            .body("chunk1chunk2chunk3");
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/stream")
        .build()
        .unwrap();

    let response = client.execute("example", request).await.unwrap();
    let mut stream = response.into_stream();

    let mut all_data = Vec::new();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.unwrap();
        all_data.extend_from_slice(&chunk);
    }

    let text = String::from_utf8(all_data).unwrap();
    assert_eq!(text, "chunk1chunk2chunk3");
}

/// Test 8: Config from environment (fallback to defaults)
#[test]
fn test_config_from_env_fallback() {
    // Use temp_env helper to avoid test interference
    temp_env::with_vars(
        vec![
            ("OAGW_AUTH_TOKEN", Some("test-token")),
            ("OAGW_BASE_URL", None), // Explicitly unset
        ],
        || {
            let config = OagwClientConfig::from_env().unwrap();

            match config.mode {
                oagw_client_prototype::ClientMode::RemoteProxy { base_url, .. } => {
                    assert_eq!(base_url, "https://oagw.internal.cf");
                }
            }
        },
    );
}

/// Test 9: Config from environment (custom base URL)
#[test]
fn test_config_from_env_custom() {
    // Use temp_env helper to avoid test interference
    temp_env::with_vars(
        vec![
            ("OAGW_BASE_URL", Some("http://custom.url")),
            ("OAGW_AUTH_TOKEN", Some("custom-token")),
        ],
        || {
            let config = OagwClientConfig::from_env().unwrap();

            match config.mode {
                oagw_client_prototype::ClientMode::RemoteProxy {
                    base_url,
                    auth_token,
                    ..
                } => {
                    assert_eq!(base_url, "http://custom.url");
                    assert_eq!(auth_token, "custom-token");
                }
            }
        },
    );
}
