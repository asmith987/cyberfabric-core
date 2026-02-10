use http::{Method, StatusCode};
use oagw_client_prototype::{
    ErrorSource, OagwClient, OagwClientConfig, Request,
};
use serde_json::json;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Test configuration
const TEST_AUTH_TOKEN: &str = "test-integration-token";
const SERVER_STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_RETRY_ATTEMPTS: u32 = 10;

/// Find an available port by binding to port 0 and getting the assigned port
fn find_available_port() -> Result<u16, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener); // Release the port
    Ok(port)
}

/// Python server process manager
struct PythonServer {
    process: Child,
    base_url: String,
    _port: u16, // Kept for debugging, not directly used
}

impl PythonServer {
    /// Start the Python server and wait for it to be ready
    fn start() -> Result<Self, Box<dyn std::error::Error>> {
        // Find an available port
        let port = find_available_port()?;

        // Check if Python is available
        let python_check = Command::new("python3")
            .arg("--version")
            .output();

        if python_check.is_err() {
            eprintln!("WARNING: Python 3 is not available. Skipping Python integration tests.");
            eprintln!("To run these tests, install Python 3 and Flask:");
            eprintln!("  cd tests/python_integration");
            eprintln!("  pip install -r requirements.txt");
            return Err("Python 3 not available".into());
        }

        println!("Starting Python OAGW mock server on port {}...", port);

        // Spawn Python server
        let server_path = std::env::current_dir()?
            .join("tests")
            .join("python_integration")
            .join("server.py");

        let process = Command::new("python3")
            .arg(server_path)
            .arg(port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let base_url = format!("http://127.0.0.1:{}", port);

        let server = PythonServer { process, base_url, _port: port };

        // Wait for server to be ready
        server.wait_until_ready()?;

        println!("Python server is ready!");

        Ok(server)
    }

    /// Wait for the server to become ready by checking the health endpoint
    fn wait_until_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        let health_url = format!("{}/health", self.base_url);

        for attempt in 1..=REQUEST_RETRY_ATTEMPTS {
            thread::sleep(Duration::from_millis(500));

            // Spawn a separate thread to avoid "runtime within runtime" issues
            // This is necessary because this sync function may be called from
            // within an existing tokio runtime context (#[tokio::test])
            let health_url_clone = health_url.clone();
            let result = thread::spawn(move || -> Result<reqwest::Response, String> {
                let runtime = tokio::runtime::Runtime::new()
                    .map_err(|e| format!("Failed to create runtime: {}", e))?;
                let response = runtime.block_on(async {
                    let client = reqwest::Client::new();
                    client.get(&health_url_clone).send().await
                })
                .map_err(|e| format!("Health check failed: {}", e))?;
                Ok(response)
            })
            .join()
            .map_err(|_| "Health check thread panicked")?
            .map_err(|e: String| -> Box<dyn std::error::Error> { e.into() })?;

            if result.status().is_success() {
                println!("Server health check passed on attempt {}", attempt);
                return Ok(());
            } else if attempt == REQUEST_RETRY_ATTEMPTS {
                return Err(format!(
                    "Server did not become ready within {:?}",
                    SERVER_STARTUP_TIMEOUT
                )
                .into());
            }
        }

        Ok(())
    }

    /// Get the base URL of the server
    fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Drop for PythonServer {
    fn drop(&mut self) {
        println!("Shutting down Python server...");
        _ = self.process.kill();
        _ = self.process.wait();
    }
}

/// Helper to create OAGW client for testing
fn create_test_client(server: &PythonServer) -> OagwClient {
    let config = OagwClientConfig::remote(
        server.base_url().to_string(),
        TEST_AUTH_TOKEN.to_string(),
    );
    OagwClient::from_config(config).expect("Failed to create client")
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_python_openai_chat_completion() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(e) => {
            println!("Skipping test - Python server not available: {e}");
            return;
        }
    };

    let client = create_test_client(&server);

    let request = Request::builder()
        .method(Method::POST)
        .path("/v1/chat/completions")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .unwrap()
        .build()
        .unwrap();

    let response = client.execute("openai", request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.error_source(), ErrorSource::Upstream);

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["object"], "chat.completion");
    assert!(data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap()
        .contains("Hello"));
}

#[tokio::test]
async fn test_python_openai_streaming() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    let request = Request::builder()
        .method(Method::POST)
        .path("/v1/chat/completions")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "test"}],
            "stream": true
        }))
        .unwrap()
        .build()
        .unwrap();

    let response = client.execute("openai", request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.error_source(), ErrorSource::Upstream);

    let mut sse = response.into_sse_stream();
    let mut chunk_count = 0;
    let mut accumulated_text = String::new();

    while let Some(event) = sse.next_event().await.unwrap() {
        if event.data.contains("[DONE]") {
            break;
        }

        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        if let Some(content) = data["choices"][0]["delta"]["content"].as_str() {
            accumulated_text.push_str(content);
        }

        chunk_count += 1;
    }

    println!("Received {} chunks, accumulated text: {}", chunk_count, accumulated_text);
    assert!(chunk_count > 0, "Should receive at least one chunk");
    assert!(!accumulated_text.is_empty(), "Should accumulate some text");
}

#[tokio::test]
async fn test_python_github_api() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    let request = Request::builder()
        .method(Method::GET)
        .path("/users/octocat")
        .build()
        .unwrap();

    let response = client.execute("github", request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.error_source(), ErrorSource::Upstream);

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["login"], "octocat");
    assert_eq!(data["name"], "The Octocat");
    assert_eq!(data["company"], "GitHub");
}

#[tokio::test]
async fn test_python_github_not_found() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    let request = Request::builder()
        .method(Method::GET)
        .path("/users/nonexistent-user-12345")
        .build()
        .unwrap();

    let response = client.execute("github", request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.error_source(), ErrorSource::Upstream);

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["message"], "Not Found");
}

#[tokio::test]
async fn test_python_weather_api() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    let request = Request::builder()
        .method(Method::GET)
        .path("/current?city=Seattle")
        .build()
        .unwrap();

    let response = client.execute("weather", request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.error_source(), ErrorSource::Upstream);

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["location"], "Seattle");
    assert_eq!(data["temperature"], 72);
    assert_eq!(data["unit"], "fahrenheit");
    assert_eq!(data["condition"], "sunny");
}

#[tokio::test]
async fn test_python_calculator_api() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    // Test addition
    let request = Request::builder()
        .method(Method::POST)
        .path("/calculate")
        .json(&json!({
            "operation": "add",
            "a": 15,
            "b": 27
        }))
        .unwrap()
        .build()
        .unwrap();

    let response = client.execute("calculator", request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["result"], 42);

    // Test division
    let request = Request::builder()
        .method(Method::POST)
        .path("/calculate")
        .json(&json!({
            "operation": "divide",
            "a": 100,
            "b": 4
        }))
        .unwrap()
        .build()
        .unwrap();

    let response = client.execute("calculator", request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["result"], 25.0);
}

#[tokio::test]
async fn test_python_calculator_error() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    // Test division by zero
    let request = Request::builder()
        .method(Method::POST)
        .path("/calculate")
        .json(&json!({
            "operation": "divide",
            "a": 100,
            "b": 0
        }))
        .unwrap()
        .build()
        .unwrap();

    let response = client.execute("calculator", request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response.error_source(), ErrorSource::Upstream);

    let data: serde_json::Value = response.json().await.unwrap();
    assert!(data["error"].as_str().unwrap().contains("Division by zero"));
}

#[tokio::test]
async fn test_python_sse_events() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    let request = Request::builder()
        .method(Method::GET)
        .path("/events")
        .build()
        .unwrap();

    let response = client.execute("stream-test", request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let mut sse = response.into_sse_stream();
    let mut events = Vec::new();

    while let Some(event) = sse.next_event().await.unwrap() {
        events.push(event);
        if events.len() >= 4 {
            break;
        }
    }

    assert_eq!(events.len(), 4);

    // Check first event
    assert_eq!(events[0].id.as_deref(), Some("1"));
    assert_eq!(events[0].event.as_deref(), Some("message"));
    assert_eq!(events[0].data, "First event");

    // Check second event
    assert_eq!(events[1].id.as_deref(), Some("2"));
    assert_eq!(events[1].event.as_deref(), Some("update"));

    // Check third event (multiline)
    assert_eq!(events[2].id.as_deref(), Some("3"));
    assert!(events[2].data.contains("multiple lines"));

    // Check fourth event
    assert_eq!(events[3].id.as_deref(), Some("4"));
    assert_eq!(events[3].event.as_deref(), Some("complete"));
}

#[tokio::test]
async fn test_python_gateway_error() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    // Test with unknown alias
    let request = Request::builder()
        .method(Method::GET)
        .path("/test")
        .build()
        .unwrap();

    let response = client.execute("unknown-alias", request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.error_source(), ErrorSource::Gateway);

    let data: serde_json::Value = response.json().await.unwrap();
    assert!(data["error"].as_str().unwrap().contains("Unknown alias"));
}

#[tokio::test]
async fn test_python_upstream_error() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    // Trigger rate limit error from OpenAI mock
    let request = Request::builder()
        .method(Method::POST)
        .path("/v1/chat/completions")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "trigger error"}]
        }))
        .unwrap()
        .build()
        .unwrap();

    let response = client.execute("openai", request).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.error_source(), ErrorSource::Upstream);
}

#[tokio::test]
async fn test_python_gateway_status() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    // This test doesn't use the standard client because it's testing
    // a gateway endpoint, not a proxied service endpoint
    let base_url = server.base_url();
    let status_url = format!("{}/api/oagw/v1/status", base_url);

    let http_client = reqwest::Client::new();
    let response = http_client
        .get(&status_url)
        .header("Authorization", format!("Bearer {}", TEST_AUTH_TOKEN))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["status"], "operational");
    assert!(data["registered_aliases"].is_array());

    let aliases = data["registered_aliases"].as_array().unwrap();
    assert!(aliases.contains(&json!("openai")));
    assert!(aliases.contains(&json!("github")));
    assert!(aliases.contains(&json!("weather")));
}

#[tokio::test]
async fn test_python_multiple_requests() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    // Make multiple requests to ensure connection reuse works
    for i in 1..=5 {
        let request = Request::builder()
            .method(Method::GET)
            .path(&format!("/users/octocat"))
            .build()
            .unwrap();

        let response = client.execute("github", request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let data: serde_json::Value = response.json().await.unwrap();
        assert_eq!(data["login"], "octocat");

        println!("Request {} completed successfully", i);
    }
}

#[tokio::test]
async fn test_python_binary_response() {
    let server = match PythonServer::start() {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test - Python server not available");
            return;
        }
    };

    let client = create_test_client(&server);

    // The weather API returns JSON, but we can consume it as bytes
    let request = Request::builder()
        .method(Method::GET)
        .path("/current?city=Boston")
        .build()
        .unwrap();

    let response = client.execute("weather", request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.bytes().await.unwrap();
    assert!(!bytes.is_empty());

    // Should be valid JSON
    let data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(data["location"], "Boston");
}
