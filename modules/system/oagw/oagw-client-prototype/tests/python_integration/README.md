# Python Integration Test Server

This directory contains a Python-based mock OAGW gateway server for integration testing the Rust OAGW client library.

## Overview

The Python server (`server.py`) simulates:
1. **OAGW Gateway** - Handles proxy routing with authentication
2. **Mock External APIs** - Simulates OpenAI, GitHub, Weather, Calculator, and streaming services

## Features

### Mock Services

- **OpenAI API** - Chat completions (buffered and streaming), model listing
- **GitHub API** - User and repository information
- **Weather API** - Current weather data
- **Calculator API** - Basic arithmetic operations
- **Stream Test API** - SSE events with metadata

### OAGW Features

- Bearer token authentication
- Error source tracking (gateway vs upstream)
- SSE streaming support
- JSON request/response handling
- Custom headers

## Setup

```bash
cd tests/python_integration
python3 -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install -r requirements.txt
```

## Running Manually

```bash
# Start server on default port (8765)
python server.py

# Or specify a custom port
python server.py 9000
```

## API Examples

### Health Check
```bash
curl http://localhost:8765/health
```

### Gateway Status
```bash
curl -H "Authorization: Bearer test-integration-token" \
  http://localhost:8765/api/oagw/v1/status
```

### OpenAI Chat Completion
```bash
curl -X POST \
  -H "Authorization: Bearer test-integration-token" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"hello"}]}' \
  http://localhost:8765/api/oagw/v1/proxy/openai/v1/chat/completions
```

### GitHub User Info
```bash
curl -H "Authorization: Bearer test-integration-token" \
  http://localhost:8765/api/oagw/v1/proxy/github/users/octocat
```

### Weather Data
```bash
curl -H "Authorization: Bearer test-integration-token" \
  "http://localhost:8765/api/oagw/v1/proxy/weather/current?city=Seattle"
```

### Calculator
```bash
curl -X POST \
  -H "Authorization: Bearer test-integration-token" \
  -H "Content-Type: application/json" \
  -d '{"operation":"add","a":5,"b":3}' \
  http://localhost:8765/api/oagw/v1/proxy/calculator/calculate
```

### SSE Streaming
```bash
curl -H "Authorization: Bearer test-integration-token" \
  http://localhost:8765/api/oagw/v1/proxy/stream-test/events
```

## Integration with Rust Tests

The Rust integration test (`tests/python_integration_test.rs`) automatically:
1. Checks for Python and required packages
2. Spawns the server on an available port
3. Runs comprehensive tests
4. Shuts down the server after tests complete

## Authentication

The mock server uses a single test token: `test-integration-token`

Invalid tokens will receive a 403 Forbidden response with `X-OAGW-Error-Source: gateway`.

## Error Source Tracking

All responses include the `X-OAGW-Error-Source` header:
- `gateway` - Error from OAGW itself (auth, routing, etc.)
- `upstream` - Error from the external service

## Endpoints

| Service | Endpoint | Methods | Description |
|---------|----------|---------|-------------|
| Health | `/health` | GET | Server health check |
| Gateway Status | `/api/oagw/v1/status` | GET | Gateway operational status |
| OpenAI Chat | `/api/oagw/v1/proxy/openai/v1/chat/completions` | POST | Chat completions |
| OpenAI Models | `/api/oagw/v1/proxy/openai/v1/models` | GET | List available models |
| GitHub User | `/api/oagw/v1/proxy/github/users/{username}` | GET | Get user info |
| GitHub Repo | `/api/oagw/v1/proxy/github/repos/{owner}/{repo}` | GET | Get repo info |
| Weather | `/api/oagw/v1/proxy/weather/current?city={city}` | GET | Get weather data |
| Calculator | `/api/oagw/v1/proxy/calculator/calculate` | POST | Perform calculation |
| Stream Test | `/api/oagw/v1/proxy/stream-test/events` | GET | SSE event stream |
