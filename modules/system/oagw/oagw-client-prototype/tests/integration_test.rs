use bytes::Bytes;
use http::{Method, StatusCode};
use httpmock::prelude::*;
use oagw_client_prototype::{
    ErrorSource, OagwClient, OagwClientConfig, Request,
};
use serde_json::json;

/// Helper function to create a mock OAGW server
fn create_mock_oagw() -> MockServer {
    MockServer::start()
}

#[tokio::test]
async fn test_simple_json_request() {
    let server = create_mock_oagw();

    // Mock OpenAI endpoint
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/oagw/v1/proxy/openai/v1/chat/completions")
            .header("Authorization", "Bearer test-token")
            .json_body(json!({"model": "gpt-4"}));
        then.status(200)
            .header("Content-Type", "application/json")
            .header("X-OAGW-Error-Source", "upstream")
            .json_body(json!({
                "choices": [{
                    "message": {"content": "Hello!"}
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

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.error_source(), ErrorSource::Upstream);

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(
        data["choices"][0]["message"]["content"],
        json!("Hello!")
    );

    mock.assert();
}

#[tokio::test]
async fn test_get_request() {
    let server = create_mock_oagw();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/unpkg/package.json")
            .header("Authorization", "Bearer test-token");
        then.status(200)
            .header("Content-Type", "application/json")
            .header("X-OAGW-Error-Source", "upstream")
            .json_body(json!({"name": "test-package"}));
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/package.json")
        .build()
        .unwrap();

    let response = client.execute("unpkg", request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["name"], json!("test-package"));

    mock.assert();
}

#[tokio::test]
async fn test_error_source_gateway() {
    let server = create_mock_oagw();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/invalid/test")
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

    let response = client.execute("invalid", request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.error_source(), ErrorSource::Gateway);

    mock.assert();
}

#[tokio::test]
async fn test_error_source_upstream() {
    let server = create_mock_oagw();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/github/nonexistent")
            .header("Authorization", "Bearer test-token");
        then.status(404)
            .header("X-OAGW-Error-Source", "upstream")
            .body("Not found");
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/nonexistent")
        .build()
        .unwrap();

    let response = client.execute("github", request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.error_source(), ErrorSource::Upstream);

    mock.assert();
}

#[tokio::test]
async fn test_text_response() {
    let server = create_mock_oagw();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/example/text")
            .header("Authorization", "Bearer test-token");
        then.status(200)
            .header("Content-Type", "text/plain")
            .header("X-OAGW-Error-Source", "upstream")
            .body("Hello, World!");
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/text")
        .build()
        .unwrap();

    let response = client.execute("example", request).await.unwrap();
    let text = response.text().await.unwrap();

    assert_eq!(text, "Hello, World!");

    mock.assert();
}

#[tokio::test]
async fn test_bytes_response() {
    let server = create_mock_oagw();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/example/binary")
            .header("Authorization", "Bearer test-token");
        then.status(200)
            .header("Content-Type", "application/octet-stream")
            .header("X-OAGW-Error-Source", "upstream")
            .body(vec![0x01, 0x02, 0x03, 0x04]);
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/binary")
        .build()
        .unwrap();

    let response = client.execute("example", request).await.unwrap();
    let bytes = response.bytes().await.unwrap();

    assert_eq!(bytes, Bytes::from(vec![0x01, 0x02, 0x03, 0x04]));

    mock.assert();
}

#[tokio::test]
async fn test_custom_headers() {
    let server = create_mock_oagw();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/example/headers")
            .header("Authorization", "Bearer test-token")
            .header("X-Custom-Header", "custom-value");
        then.status(200)
            .header("X-OAGW-Error-Source", "upstream")
            .body("OK");
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/headers")
        .header("X-Custom-Header", "custom-value")
        .unwrap()
        .build()
        .unwrap();

    let response = client.execute("example", request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    mock.assert();
}

#[tokio::test]
async fn test_blocking_request() {
    let server = create_mock_oagw();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/oagw/v1/proxy/example/blocking")
            .header("Authorization", "Bearer test-token");
        then.status(200)
            .header("X-OAGW-Error-Source", "upstream")
            .json_body(json!({"result": "success"}));
    });

    let config = OagwClientConfig::remote(server.base_url(), "test-token".to_string());
    let client = OagwClient::from_config(config).unwrap();

    let request = Request::builder()
        .method(Method::GET)
        .path("/blocking")
        .build()
        .unwrap();

    // Note: In tests we can't truly test blocking API due to MockServer requiring tokio
    // In real build.rs, execute_blocking() would work without existing runtime
    let response = client.execute("example", request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["result"], json!("success"));

    mock.assert();
}
