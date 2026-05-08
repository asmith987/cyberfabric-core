//! Type-safe API operation builder with compile-time guarantees
//!
//! This module provides a type-state builder pattern that enforces at compile time
//! that API operations cannot be registered unless both a handler and at least one
//! response are specified.

pub mod api_dto;
pub mod error_layer;
pub mod odata;
pub mod openapi_registry;
pub mod operation_builder;
pub mod response;
pub mod select;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod odata_policy_tests;

pub use error_layer::error_middleware;
pub use openapi_registry::{OpenApiInfo, OpenApiRegistry, OpenApiRegistryImpl, ensure_schema};
pub use operation_builder::{
    Missing, OperationBuilder, OperationSpec, ParamLocation, ParamSpec, Present, RateLimitSpec,
    ResponseSpec, state,
};
pub use select::{apply_select, page_to_projected_json, project_json};

/// Prelude for modules using the canonical error catalog.
pub mod prelude {
    // Canonical error types
    pub use modkit_errors::{CanonicalError, Problem, resource_error};

    /// Result type alias for handlers using the canonical error catalog.
    ///
    /// Returns [`CanonicalError`] (not [`Problem`]) so handler `?` chains
    /// resolve through `From<DomainError> for CanonicalError` — the
    /// long-lived per-module mapping. The canonical error middleware
    /// (`modkit::api::error_middleware`) converts the
    /// `CanonicalError` to a wire `Problem` and fills `instance` /
    /// `trace_id` on the way out, so handlers never need to construct a
    /// `Problem` themselves.
    pub type ApiResult<T = ()> = std::result::Result<T, CanonicalError>;

    // Same response sugar / OData / axum re-exports as the legacy prelude
    pub use super::response::{JsonBody, JsonPage, created_json, no_content, ok_json};
    pub use super::select::apply_select;
    pub use axum::{Json, http::StatusCode, response::IntoResponse};
}
