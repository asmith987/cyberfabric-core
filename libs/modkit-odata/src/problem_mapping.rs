//! Mapping from `OData` errors to canonical [`CanonicalError`].
//!
//! The HTTP layer in `modkit` (`api::odata::error::odata_error_to_problem`)
//! converts the resulting `CanonicalError` to a wire `Problem`, attaching
//! `instance` and `trace_id` from request context.

use modkit_errors::CanonicalError;

use crate::Error;
use crate::errors::OdataError;

impl From<Error> for CanonicalError {
    fn from(err: Error) -> Self {
        use Error::{
            CursorInvalidBase64, CursorInvalidDirection, CursorInvalidFields, CursorInvalidJson,
            CursorInvalidKeys, CursorInvalidVersion, Db, FilterMismatch, InvalidCursor,
            InvalidFilter, InvalidLimit, InvalidOrderByField, OrderMismatch, OrderWithCursor,
            ParsingUnavailable,
        };

        match err {
            InvalidFilter(msg) => OdataError::invalid_argument()
                .with_field_violation(
                    "$filter",
                    format!("Invalid $filter: {msg}"),
                    "INVALID_FILTER",
                )
                .create(),

            InvalidOrderByField(field) => OdataError::invalid_argument()
                .with_field_violation(
                    "$orderby",
                    format!("Unsupported $orderby field: {field}"),
                    "INVALID_ORDERBY_FIELD",
                )
                .create(),

            InvalidCursor => OdataError::invalid_argument()
                .with_field_violation("cursor", "invalid cursor", "INVALID_CURSOR")
                .create(),

            CursorInvalidBase64 => OdataError::invalid_argument()
                .with_field_violation(
                    "cursor",
                    "invalid cursor: invalid base64url encoding",
                    "INVALID_CURSOR",
                )
                .create(),

            CursorInvalidJson => OdataError::invalid_argument()
                .with_field_violation("cursor", "invalid cursor: malformed JSON", "INVALID_CURSOR")
                .create(),

            CursorInvalidVersion => OdataError::invalid_argument()
                .with_field_violation(
                    "cursor",
                    "invalid cursor: unsupported version",
                    "INVALID_CURSOR",
                )
                .create(),

            CursorInvalidKeys => OdataError::invalid_argument()
                .with_field_violation(
                    "cursor",
                    "invalid cursor: empty or invalid keys",
                    "INVALID_CURSOR",
                )
                .create(),

            CursorInvalidFields => OdataError::invalid_argument()
                .with_field_violation(
                    "cursor",
                    "invalid cursor: empty or invalid fields",
                    "INVALID_CURSOR",
                )
                .create(),

            CursorInvalidDirection => OdataError::invalid_argument()
                .with_field_violation(
                    "cursor",
                    "invalid cursor: invalid sort direction",
                    "INVALID_CURSOR",
                )
                .create(),

            OrderMismatch => OdataError::invalid_argument()
                .with_field_violation(
                    "cursor",
                    "Order mismatch between cursor and query",
                    "ORDER_MISMATCH",
                )
                .create(),

            FilterMismatch => OdataError::invalid_argument()
                .with_field_violation(
                    "cursor",
                    "Filter mismatch between cursor and query",
                    "FILTER_MISMATCH",
                )
                .create(),

            InvalidLimit => OdataError::invalid_argument()
                .with_field_violation("$top", "Invalid limit parameter", "INVALID_LIMIT")
                .create(),

            OrderWithCursor => OdataError::invalid_argument()
                .with_field_violation(
                    "$orderby",
                    "Cannot specify both $orderby and cursor parameters",
                    "ORDER_WITH_CURSOR",
                )
                .create(),

            Db(msg) => {
                tracing::error!(error = %msg, "Unexpected database error in OData layer");
                CanonicalError::internal(
                    "An internal error occurred while processing the OData query",
                )
                .create()
            }

            ParsingUnavailable(msg) => {
                tracing::error!(error = %msg, "OData parsing unavailable");
                CanonicalError::internal(format!("OData parsing unavailable: {msg}")).create()
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use modkit_errors::Problem;

    fn wire(err: Error) -> Problem {
        Problem::from(CanonicalError::from(err))
    }

    #[test]
    fn invalid_filter_emits_invalid_argument() {
        let p = wire(Error::InvalidFilter("malformed".into()));
        assert_eq!(p.status, 400);
        assert!(p.problem_type.contains("invalid_argument"));
    }

    #[test]
    fn orderby_field_emits_invalid_argument() {
        let p = wire(Error::InvalidOrderByField("unknown".into()));
        assert_eq!(p.status, 400);
        assert!(p.problem_type.contains("invalid_argument"));
    }

    #[test]
    fn cursor_error_emits_invalid_argument() {
        let p = wire(Error::CursorInvalidBase64);
        assert_eq!(p.status, 400);
        assert!(p.problem_type.contains("invalid_argument"));
    }
}
