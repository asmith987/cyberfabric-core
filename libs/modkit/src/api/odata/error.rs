//! Centralized `OData` error mapping
//!
//! This module adds HTTP-specific context (instance path, trace ID) to `OData`
//! errors. The core `Error → CanonicalError` mapping is owned by `modkit-odata`;
//! here we convert `CanonicalError` to a wire `Problem` and attach
//! request-scoped fields.

use modkit_canonical_errors::{CanonicalError, Problem};
use modkit_odata::Error as ODataError;

#[inline]
fn current_trace_id() -> Option<String> {
    tracing::Span::current()
        .id()
        .map(|id| id.into_u64().to_string())
}

/// Returns a fully contextualized canonical `Problem` for `OData` errors.
///
/// The `instance` parameter should be the request path. `trace_id` defaults
/// to the current tracing span when `None`. Server-side diagnostics for
/// `Db` and `ParsingUnavailable` are emitted by `From<Error> for
/// CanonicalError` in `modkit-odata` (the single source of truth) — this
/// bridge only attaches request-scoped fields.
pub fn odata_error_to_problem(
    err: &ODataError,
    instance: &str,
    trace_id: Option<String>,
) -> Problem {
    let canonical = CanonicalError::from(err.clone());
    let mut problem = Problem::from(canonical);
    problem.instance = Some(instance.to_owned());
    problem.trace_id = trace_id.or_else(current_trace_id);
    problem
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_filter_error_mapping() {
        let error = ODataError::InvalidFilter("malformed expression".to_owned());
        let problem = odata_error_to_problem(&error, "/user-management/v1/users", None);

        assert_eq!(problem.status, 400);
        assert!(problem.problem_type.contains("invalid_argument"));
        assert_eq!(
            problem.instance,
            Some("/user-management/v1/users".to_owned())
        );
    }

    #[test]
    fn test_orderby_error_mapping() {
        let error = ODataError::InvalidOrderByField("unknown_field".to_owned());
        let problem = odata_error_to_problem(&error, "/user-management/v1/users", None);

        assert_eq!(problem.status, 400);
        assert!(problem.problem_type.contains("invalid_argument"));
    }

    #[test]
    fn test_cursor_error_mapping() {
        let error = ODataError::CursorInvalidBase64;
        let problem = odata_error_to_problem(
            &error,
            "/user-management/v1/users",
            Some("trace123".to_owned()),
        );

        assert_eq!(problem.status, 400);
        assert!(problem.problem_type.contains("invalid_argument"));
        assert_eq!(problem.trace_id, Some("trace123".to_owned()));
    }

    #[test]
    fn test_problem_type_format() {
        let error = ODataError::InvalidFilter("test".to_owned());
        let problem = odata_error_to_problem(&error, "/user-management/v1/test", None);

        // Canonical Problem.problem_type is `gts://<canonical-category>`.
        // The OData resource type lives in `context.resource_type`.
        assert!(problem.problem_type.starts_with("gts://"));
        assert!(problem.problem_type.contains("invalid_argument"));
        let rt = problem
            .context
            .get("resource_type")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(rt.contains("odata"), "resource_type was {rt:?}");
    }
}
