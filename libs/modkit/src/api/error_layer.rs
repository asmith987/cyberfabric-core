//! Centralized error mapping for Axum
//!
//! This module provides utilities for automatically converting all framework
//! and module errors into consistent RFC 9457 Problem+JSON responses, eliminating
//! per-route boilerplate.

use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};
use std::any::Any;

use crate::config::ConfigError;
use modkit_canonical_errors::{CanonicalError, Problem};
use modkit_odata::Error as ODataError;

/// Middleware function that provides centralized error mapping
///
/// This middleware can be applied to routes to automatically extract request context
/// and provide it to error handlers. The actual error conversion happens in the
/// `IntoProblem` trait implementations and `map_error_to_problem` function.
pub async fn error_mapping_middleware(request: Request, next: Next) -> Response {
    let _uri = request.uri().clone();
    let _headers = request.headers().clone();

    let response = next.run(request).await;

    // If the response is already successful or is already a Problem response, pass it through
    if response.status().is_success() || is_problem_response(&response) {
        return response;
    }

    // For error responses, the actual error conversion should happen in the handlers
    // using the IntoProblem trait or map_error_to_problem function
    // This middleware provides the infrastructure for extracting request context
    response
}

/// Check if a response is already a Problem+JSON response
fn is_problem_response(response: &Response) -> bool {
    response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.contains("application/problem+json"))
}

/// Extract trace ID from headers or generate one
pub fn extract_trace_id(headers: &HeaderMap) -> Option<String> {
    // Try to get trace ID from various common headers
    headers
        .get("x-trace-id")
        .or_else(|| headers.get("x-request-id"))
        .or_else(|| headers.get("traceparent"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .or_else(|| {
            // Try to get from current tracing span
            tracing::Span::current()
                .id()
                .map(|id| id.into_u64().to_string())
        })
}

fn internal_problem(
    detail: impl Into<String>,
    instance: &str,
    trace_id: Option<String>,
) -> Problem {
    let canonical = CanonicalError::internal(detail).create();
    let mut problem = Problem::from(canonical);
    problem.instance = Some(instance.to_owned());
    problem.trace_id = trace_id;
    problem
}

/// Centralized error mapping function
///
/// This function provides a single place to convert all framework and module errors
/// into consistent canonical `Problem` responses with proper trace IDs and instance
/// paths.
pub fn map_error_to_problem(error: &dyn Any, instance: &str, trace_id: Option<String>) -> Problem {
    if let Some(odata_err) = error.downcast_ref::<ODataError>() {
        return crate::api::odata::error::odata_error_to_problem(odata_err, instance, trace_id);
    }

    if let Some(config_err) = error.downcast_ref::<ConfigError>() {
        let detail = match config_err {
            ConfigError::ModuleNotFound { module } => {
                format!("Module '{module}' configuration not found")
            }
            ConfigError::InvalidModuleStructure { module } => {
                format!("Module '{module}' has invalid configuration structure")
            }
            ConfigError::MissingConfigSection { module } => {
                format!("Module '{module}' is missing required config section")
            }
            ConfigError::InvalidConfig { module, .. } => {
                format!("Module '{module}' has invalid configuration")
            }
            ConfigError::VarExpand { module, source } => {
                tracing::error!(
                    module = %module,
                    error = %source,
                    "Environment variable expansion failed in module config"
                );
                format!("Module '{module}' has invalid environment-backed configuration")
            }
        };
        return internal_problem(detail, instance, trace_id);
    }

    if let Some(anyhow_err) = error.downcast_ref::<anyhow::Error>() {
        tracing::error!(error = %anyhow_err, "Internal server error");
        return internal_problem("An internal error occurred", instance, trace_id);
    }

    tracing::error!("Unknown error type in error mapping layer");
    internal_problem("An unknown error occurred", instance, trace_id)
}

/// Helper trait for converting errors to Problem responses with context
pub trait IntoProblem {
    fn into_problem(self, instance: &str, trace_id: Option<String>) -> Problem;
}

impl IntoProblem for ODataError {
    fn into_problem(self, instance: &str, trace_id: Option<String>) -> Problem {
        crate::api::odata::error::odata_error_to_problem(&self, instance, trace_id)
    }
}

impl IntoProblem for ConfigError {
    fn into_problem(self, instance: &str, trace_id: Option<String>) -> Problem {
        map_error_to_problem(&self as &dyn Any, instance, trace_id)
    }
}

impl IntoProblem for anyhow::Error {
    fn into_problem(self, instance: &str, trace_id: Option<String>) -> Problem {
        map_error_to_problem(&self as &dyn Any, instance, trace_id)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_odata_error_mapping() {
        let error = ODataError::InvalidFilter("malformed".to_owned());
        let problem = error.into_problem("/tests/v1/test", Some("trace123".to_owned()));

        assert_eq!(problem.status, 400);
        assert!(problem.problem_type.contains("invalid_argument"));
        assert_eq!(problem.instance, Some("/tests/v1/test".to_owned()));
        assert_eq!(problem.trace_id, Some("trace123".to_owned()));
    }

    #[test]
    fn test_config_error_mapping() {
        let error = ConfigError::ModuleNotFound {
            module: "test_module".to_owned(),
        };
        let problem = error.into_problem("/tests/v1/test", None);

        // Canonical `Internal` errors emit a fixed wire `detail` and stash the
        // descriptive cause in the (debug-only) diagnostic — module names are
        // intentionally not echoed on the wire.
        assert_eq!(problem.status, 500);
        assert!(problem.problem_type.contains("internal"));
        assert_eq!(problem.instance, Some("/tests/v1/test".to_owned()));
    }

    #[test]
    fn test_anyhow_error_mapping() {
        let error = anyhow::anyhow!("Something went wrong");
        let problem = error.into_problem("/tests/v1/test", Some("trace456".to_owned()));

        assert_eq!(problem.status, 500);
        assert!(problem.problem_type.contains("internal"));
        assert_eq!(problem.instance, Some("/tests/v1/test".to_owned()));
        assert_eq!(problem.trace_id, Some("trace456".to_owned()));
    }

    #[test]
    fn test_config_var_expand_error_sanitizes_detail() {
        let source = modkit_utils::var_expand::ExpandVarsError::Var {
            name: "SECRET_API_KEY".to_owned(),
            source: std::env::VarError::NotPresent,
        };
        let error = ConfigError::VarExpand {
            module: "my_mod".to_owned(),
            source,
        };
        let problem = error.into_problem("/tests/v1/test", Some("trace789".to_owned()));

        assert_eq!(problem.status, 500);
        assert!(problem.problem_type.contains("internal"));
        assert_eq!(problem.instance, Some("/tests/v1/test".to_owned()));
        assert_eq!(problem.trace_id, Some("trace789".to_owned()));

        // Detail MUST NOT leak the env var name or the underlying error message.
        assert!(
            !problem.detail.contains("SECRET_API_KEY"),
            "detail must not contain env var name, got: {}",
            problem.detail,
        );
        assert!(
            !problem.detail.contains("not present"),
            "detail must not contain source error text, got: {}",
            problem.detail,
        );
    }

    #[test]
    fn test_extract_trace_id_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-trace-id", "test-trace-123".parse().unwrap());

        let trace_id = extract_trace_id(&headers);
        assert_eq!(trace_id, Some("test-trace-123".to_owned()));
    }
}
