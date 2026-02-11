use std::pin::Pin;

use bytes::Bytes;
use futures_core::Stream;
use http::{HeaderMap, Method, StatusCode};
use uuid::Uuid;

use crate::error::ServiceGatewayError;
use crate::{
    CreateRouteRequest, CreateUpstreamRequest, ListQuery, Route, UpdateRouteRequest,
    UpdateUpstreamRequest, Upstream,
};

// ---------------------------------------------------------------------------
// Body / Error aliases
// ---------------------------------------------------------------------------

/// Boxed error type for body stream errors.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// A streaming response body.
pub type BodyStream = Pin<Box<dyn Stream<Item = Result<Bytes, BoxError>> + Send>>;

// ---------------------------------------------------------------------------
// Proxy types
// ---------------------------------------------------------------------------

/// Distinguishes gateway-originated errors from upstream-originated errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSource {
    Gateway,
    Upstream,
}

/// Context for a proxy request flowing through the Data Plane.
pub struct ProxyContext {
    pub tenant_id: Uuid,
    pub method: Method,
    pub alias: String,
    pub path_suffix: String,
    pub query_params: Vec<(String, String)>,
    pub headers: HeaderMap,
    /// Request body (already validated for size).
    pub body: Bytes,
    /// Original request URI for error reporting.
    pub instance_uri: String,
}

/// Response from the Data Plane proxy pipeline.
pub struct ProxyResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: BodyStream,
    pub error_source: ErrorSource,
}

// ---------------------------------------------------------------------------
// Service trait
// ---------------------------------------------------------------------------

/// Public API trait for the Outbound API Gateway (Version 1).
///
/// This trait is registered in `ClientHub` by the OAGW module:
/// ```ignore
/// let gw = hub.get::<dyn ServiceGatewayClientV1>()?;
/// ```
#[async_trait::async_trait]
pub trait ServiceGatewayClientV1: Send + Sync {
    // -- Upstream CRUD --

    async fn create_upstream(
        &self,
        tenant_id: Uuid,
        req: CreateUpstreamRequest,
    ) -> Result<Upstream, ServiceGatewayError>;

    async fn get_upstream(&self, tenant_id: Uuid, id: Uuid) -> Result<Upstream, ServiceGatewayError>;

    async fn list_upstreams(
        &self,
        tenant_id: Uuid,
        query: &ListQuery,
    ) -> Result<Vec<Upstream>, ServiceGatewayError>;

    async fn update_upstream(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        req: UpdateUpstreamRequest,
    ) -> Result<Upstream, ServiceGatewayError>;

    async fn delete_upstream(&self, tenant_id: Uuid, id: Uuid) -> Result<(), ServiceGatewayError>;

    // -- Route CRUD --

    async fn create_route(
        &self,
        tenant_id: Uuid,
        req: CreateRouteRequest,
    ) -> Result<Route, ServiceGatewayError>;

    async fn get_route(&self, tenant_id: Uuid, id: Uuid) -> Result<Route, ServiceGatewayError>;

    async fn list_routes(
        &self,
        tenant_id: Uuid,
        upstream_id: Uuid,
        query: &ListQuery,
    ) -> Result<Vec<Route>, ServiceGatewayError>;

    async fn update_route(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        req: UpdateRouteRequest,
    ) -> Result<Route, ServiceGatewayError>;

    async fn delete_route(&self, tenant_id: Uuid, id: Uuid) -> Result<(), ServiceGatewayError>;

    // -- Resolution --

    /// Resolve an upstream by alias. Returns UpstreamDisabled if the upstream exists but is disabled.
    async fn resolve_upstream(&self, tenant_id: Uuid, alias: &str) -> Result<Upstream, ServiceGatewayError>;

    /// Find the best matching route for the given method and path under an upstream.
    async fn resolve_route(
        &self,
        tenant_id: Uuid,
        upstream_id: Uuid,
        method: &str,
        path: &str,
    ) -> Result<Route, ServiceGatewayError>;

    // -- Proxy --

    /// Execute the full proxy pipeline: resolve -> auth -> rate-limit -> forward -> respond.
    async fn proxy_request(&self, ctx: ProxyContext) -> Result<ProxyResponse, ServiceGatewayError>;
}
