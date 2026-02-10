#!/usr/bin/env python3
"""
Python-based OAGW Gateway Mock Server for Integration Testing

This server simulates both:
1. An OAGW gateway that routes requests to external services
2. Mock external APIs (e.g., OpenAI, GitHub, custom services)

The server provides realistic testing scenarios for the Rust OAGW client library.
"""

import json
import time
from flask import Flask, request, Response, jsonify, stream_with_context
import sys

app = Flask(__name__)

# Configuration
VALID_TOKEN = "test-integration-token"

# Mock data stores
chat_history = []
github_repos = {
    "octocat": {
        "login": "octocat",
        "id": 1,
        "type": "User",
        "name": "The Octocat",
        "company": "GitHub",
        "blog": "https://github.blog"
    }
}


def check_auth():
    """Validate authorization token"""
    auth_header = request.headers.get("Authorization", "")
    if not auth_header.startswith("Bearer "):
        return False, jsonify({"error": "Missing or invalid authorization"}), 401

    token = auth_header.replace("Bearer ", "")
    if token != VALID_TOKEN:
        return False, jsonify({"error": "Invalid token"}), 403

    return True, None, None


def add_error_source_header(response, source="upstream"):
    """Add X-OAGW-Error-Source header to response"""
    response.headers["X-OAGW-Error-Source"] = source
    return response


# ============================================================================
# OAGW Proxy Endpoints
# ============================================================================

@app.route("/api/oagw/v1/proxy/<alias>/<path:service_path>", methods=["GET", "POST", "PUT", "DELETE"])
def proxy_request(alias, service_path):
    """
    Main OAGW proxy endpoint that routes requests to mock external services
    """
    # Check authorization
    is_valid, error_response, status_code = check_auth()
    if not is_valid:
        response = add_error_source_header(Response(
            json.dumps({"error": "Unauthorized"}),
            status=status_code,
            mimetype="application/json"
        ), "gateway")
        return response

    # Route to appropriate service handler
    if alias == "openai":
        return handle_openai(service_path)
    elif alias == "github":
        return handle_github(service_path)
    elif alias == "weather":
        return handle_weather(service_path)
    elif alias == "calculator":
        return handle_calculator(service_path)
    elif alias == "stream-test":
        return handle_stream_test(service_path)
    else:
        response = add_error_source_header(Response(
            json.dumps({"error": f"Unknown alias: {alias}"}),
            status=404,
            mimetype="application/json"
        ), "gateway")
        return response


# ============================================================================
# Mock OpenAI API
# ============================================================================

def handle_openai(path):
    """Handle OpenAI API requests"""
    if path == "v1/chat/completions":
        return handle_chat_completions()
    elif path == "v1/models":
        return handle_list_models()
    else:
        response = add_error_source_header(Response(
            json.dumps({"error": "Not found"}),
            status=404,
            mimetype="application/json"
        ), "upstream")
        return response


def handle_chat_completions():
    """Handle chat completions (both streaming and non-streaming)"""
    try:
        data = request.get_json()
        model = data.get("model", "gpt-4")
        messages = data.get("messages", [])
        stream = data.get("stream", False)

        # Store in history
        chat_history.append({
            "timestamp": time.time(),
            "model": model,
            "messages": messages
        })

        if stream:
            return handle_streaming_chat(model, messages)
        else:
            return handle_buffered_chat(model, messages)
    except Exception as e:
        response = add_error_source_header(Response(
            json.dumps({"error": str(e)}),
            status=400,
            mimetype="application/json"
        ), "upstream")
        return response


def handle_buffered_chat(model, messages):
    """Handle non-streaming chat completion"""
    last_message = messages[-1] if messages else {"content": ""}
    user_content = last_message.get("content", "")

    # Generate a simple response based on the prompt
    if "hello" in user_content.lower():
        response_text = "Hello! How can I assist you today?"
    elif "error" in user_content.lower():
        # Simulate an upstream error
        response = add_error_source_header(Response(
            json.dumps({"error": {"message": "Rate limit exceeded", "type": "rate_limit_error"}}),
            status=429,
            mimetype="application/json"
        ), "upstream")
        return response
    else:
        response_text = f"I received your message: '{user_content}'. This is a mock response from the Python test server."

    result = {
        "id": f"chatcmpl-{int(time.time())}",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": response_text
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 20,
            "total_tokens": 30
        }
    }

    response = add_error_source_header(
        jsonify(result),
        "upstream"
    )
    return response


def handle_streaming_chat(model, messages):
    """Handle streaming chat completion using SSE"""
    def generate():
        last_message = messages[-1] if messages else {"content": ""}
        user_content = last_message.get("content", "")

        # Generate response tokens
        response_text = f"Streaming response to: {user_content}"
        words = response_text.split()

        # Stream each word as a separate event
        for i, word in enumerate(words):
            chunk = {
                "id": f"chatcmpl-{int(time.time())}",
                "object": "chat.completion.chunk",
                "created": int(time.time()),
                "model": model,
                "choices": [{
                    "index": 0,
                    "delta": {"content": word + " "},
                    "finish_reason": None
                }]
            }
            yield f"data: {json.dumps(chunk)}\n\n"
            time.sleep(0.05)  # Small delay to simulate streaming

        # Send final chunk
        final_chunk = {
            "id": f"chatcmpl-{int(time.time())}",
            "object": "chat.completion.chunk",
            "created": int(time.time()),
            "model": model,
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        }
        yield f"data: {json.dumps(final_chunk)}\n\n"
        yield "data: [DONE]\n\n"

    response = Response(
        stream_with_context(generate()),
        mimetype="text/event-stream"
    )
    response.headers["Cache-Control"] = "no-cache"
    response.headers["X-Accel-Buffering"] = "no"
    response = add_error_source_header(response, "upstream")
    return response


def handle_list_models():
    """Handle list models request"""
    models = {
        "object": "list",
        "data": [
            {"id": "gpt-4", "object": "model", "owned_by": "openai"},
            {"id": "gpt-3.5-turbo", "object": "model", "owned_by": "openai"}
        ]
    }
    response = add_error_source_header(jsonify(models), "upstream")
    return response


# ============================================================================
# Mock GitHub API
# ============================================================================

def handle_github(path):
    """Handle GitHub API requests"""
    if path.startswith("users/"):
        username = path.replace("users/", "")
        return handle_get_user(username)
    elif path.startswith("repos/"):
        return handle_get_repo(path.replace("repos/", ""))
    else:
        response = add_error_source_header(Response(
            json.dumps({"message": "Not Found"}),
            status=404,
            mimetype="application/json"
        ), "upstream")
        return response


def handle_get_user(username):
    """Handle get user request"""
    if username in github_repos:
        response = add_error_source_header(
            jsonify(github_repos[username]),
            "upstream"
        )
        return response
    else:
        response = add_error_source_header(Response(
            json.dumps({"message": "Not Found"}),
            status=404,
            mimetype="application/json"
        ), "upstream")
        return response


def handle_get_repo(repo_path):
    """Handle get repository request"""
    response = add_error_source_header(jsonify({
        "name": repo_path,
        "full_name": repo_path,
        "description": "A mock repository for testing",
        "private": False,
        "html_url": f"https://github.com/{repo_path}"
    }), "upstream")
    return response


# ============================================================================
# Mock Weather API
# ============================================================================

def handle_weather(path):
    """Handle weather API requests"""
    if "current" in path:
        city = request.args.get("city", "Unknown")
        weather_data = {
            "location": city,
            "temperature": 72,
            "unit": "fahrenheit",
            "condition": "sunny",
            "humidity": 45,
            "wind_speed": 8
        }
        response = add_error_source_header(jsonify(weather_data), "upstream")
        return response
    else:
        response = add_error_source_header(Response(
            json.dumps({"error": "Invalid endpoint"}),
            status=404,
            mimetype="application/json"
        ), "upstream")
        return response


# ============================================================================
# Mock Calculator API
# ============================================================================

def handle_calculator(path):
    """Handle calculator API requests"""
    try:
        data = request.get_json()
        operation = data.get("operation")
        a = data.get("a", 0)
        b = data.get("b", 0)

        if operation == "add":
            result = a + b
        elif operation == "subtract":
            result = a - b
        elif operation == "multiply":
            result = a * b
        elif operation == "divide":
            if b == 0:
                response = add_error_source_header(Response(
                    json.dumps({"error": "Division by zero"}),
                    status=400,
                    mimetype="application/json"
                ), "upstream")
                return response
            result = a / b
        else:
            response = add_error_source_header(Response(
                json.dumps({"error": f"Unknown operation: {operation}"}),
                status=400,
                mimetype="application/json"
            ), "upstream")
            return response

        response = add_error_source_header(jsonify({
            "operation": operation,
            "a": a,
            "b": b,
            "result": result
        }), "upstream")
        return response
    except Exception as e:
        response = add_error_source_header(Response(
            json.dumps({"error": str(e)}),
            status=400,
            mimetype="application/json"
        ), "upstream")
        return response


# ============================================================================
# Mock Stream Test API
# ============================================================================

def handle_stream_test(path):
    """Handle stream test requests for SSE testing"""
    if "events" in path:
        def generate():
            # Send multiple SSE events with different types
            events = [
                {"id": "1", "event": "message", "data": "First event"},
                {"id": "2", "event": "update", "data": "Second event"},
                {"id": "3", "event": "message", "data": "Third event with\nmultiple lines"},
                {"id": "4", "event": "complete", "data": "Final event"}
            ]

            for evt in events:
                if "id" in evt:
                    yield f"id: {evt['id']}\n"
                if "event" in evt:
                    yield f"event: {evt['event']}\n"

                # Handle multiline data correctly in SSE format
                # Each line must be prefixed with "data: "
                data_lines = evt['data'].split('\n')
                for line in data_lines:
                    yield f"data: {line}\n"

                yield "\n"  # Empty line to terminate the event
                time.sleep(0.1)

        response = Response(
            stream_with_context(generate()),
            mimetype="text/event-stream"
        )
        response.headers["Cache-Control"] = "no-cache"
        response = add_error_source_header(response, "upstream")
        return response
    else:
        response = add_error_source_header(Response(
            json.dumps({"error": "Invalid endpoint"}),
            status=404,
            mimetype="application/json"
        ), "upstream")
        return response


# ============================================================================
# Health Check & Status
# ============================================================================

@app.route("/health", methods=["GET"])
def health_check():
    """Health check endpoint"""
    return jsonify({"status": "healthy", "service": "oagw-mock-server"})


@app.route("/api/oagw/v1/status", methods=["GET"])
def gateway_status():
    """Gateway status endpoint"""
    response = add_error_source_header(jsonify({
        "status": "operational",
        "version": "0.1.0-test",
        "uptime": time.time(),
        "registered_aliases": ["openai", "github", "weather", "calculator", "stream-test"]
    }), "gateway")
    return response


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
    print(f"Starting OAGW Mock Server on port {port}...", file=sys.stderr)
    print(f"Auth token: {VALID_TOKEN}", file=sys.stderr)
    app.run(host="127.0.0.1", port=port, debug=False, threaded=True)
