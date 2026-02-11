use std::sync::Arc;

use oagw_sdk::api::{
    ProxyContext as SdkProxyContext, ProxyResponse as SdkProxyResponse,
    ServiceGatewayClientV1,
};
use oagw_sdk::error::ServiceGatewayError;
use uuid::Uuid;

use crate::domain::dto as dto;
use crate::domain::error::DomainError;
use super::{ControlPlaneService, DataPlaneService};

/// Facade that implements the public `ServiceGatewayClientV1` trait by
/// delegating to the internal CP and DP services.
pub(crate) struct ServiceGatewayClientV1Facade {
    cp: Arc<dyn ControlPlaneService>,
    dp: Arc<dyn DataPlaneService>,
}

impl ServiceGatewayClientV1Facade {
    pub(crate) fn new(cp: Arc<dyn ControlPlaneService>, dp: Arc<dyn DataPlaneService>) -> Self {
        Self { cp, dp }
    }
}

#[async_trait::async_trait]
impl ServiceGatewayClientV1 for ServiceGatewayClientV1Facade {
    async fn create_upstream(
        &self,
        tenant_id: Uuid,
        req: oagw_sdk::CreateUpstreamRequest,
    ) -> Result<oagw_sdk::Upstream, ServiceGatewayError> {
        let internal_req = sdk_create_upstream_to_domain(req);
        let result = self.cp.create_upstream(tenant_id, internal_req).await;
        result.map(upstream_to_sdk).map_err(domain_err_to_sdk)
    }

    async fn get_upstream(&self, tenant_id: Uuid, id: Uuid) -> Result<oagw_sdk::Upstream, ServiceGatewayError> {
        self.cp.get_upstream(tenant_id, id).await.map(upstream_to_sdk).map_err(domain_err_to_sdk)
    }

    async fn list_upstreams(
        &self,
        tenant_id: Uuid,
        query: &oagw_sdk::ListQuery,
    ) -> Result<Vec<oagw_sdk::Upstream>, ServiceGatewayError> {
        let q = dto::ListQuery { top: query.top, skip: query.skip };
        self.cp.list_upstreams(tenant_id, &q).await
            .map(|v| v.into_iter().map(upstream_to_sdk).collect())
            .map_err(domain_err_to_sdk)
    }

    async fn update_upstream(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        req: oagw_sdk::UpdateUpstreamRequest,
    ) -> Result<oagw_sdk::Upstream, ServiceGatewayError> {
        let internal_req = sdk_update_upstream_to_domain(req);
        self.cp.update_upstream(tenant_id, id, internal_req).await
            .map(upstream_to_sdk).map_err(domain_err_to_sdk)
    }

    async fn delete_upstream(&self, tenant_id: Uuid, id: Uuid) -> Result<(), ServiceGatewayError> {
        self.cp.delete_upstream(tenant_id, id).await.map_err(domain_err_to_sdk)
    }

    async fn create_route(
        &self,
        tenant_id: Uuid,
        req: oagw_sdk::CreateRouteRequest,
    ) -> Result<oagw_sdk::Route, ServiceGatewayError> {
        let internal_req = sdk_create_route_to_domain(req);
        self.cp.create_route(tenant_id, internal_req).await
            .map(route_to_sdk).map_err(domain_err_to_sdk)
    }

    async fn get_route(&self, tenant_id: Uuid, id: Uuid) -> Result<oagw_sdk::Route, ServiceGatewayError> {
        self.cp.get_route(tenant_id, id).await.map(route_to_sdk).map_err(domain_err_to_sdk)
    }

    async fn list_routes(
        &self,
        tenant_id: Uuid,
        upstream_id: Uuid,
        query: &oagw_sdk::ListQuery,
    ) -> Result<Vec<oagw_sdk::Route>, ServiceGatewayError> {
        let q = dto::ListQuery { top: query.top, skip: query.skip };
        self.cp.list_routes(tenant_id, upstream_id, &q).await
            .map(|v| v.into_iter().map(route_to_sdk).collect())
            .map_err(domain_err_to_sdk)
    }

    async fn update_route(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        req: oagw_sdk::UpdateRouteRequest,
    ) -> Result<oagw_sdk::Route, ServiceGatewayError> {
        let internal_req = sdk_update_route_to_domain(req);
        self.cp.update_route(tenant_id, id, internal_req).await
            .map(route_to_sdk).map_err(domain_err_to_sdk)
    }

    async fn delete_route(&self, tenant_id: Uuid, id: Uuid) -> Result<(), ServiceGatewayError> {
        self.cp.delete_route(tenant_id, id).await.map_err(domain_err_to_sdk)
    }

    async fn resolve_upstream(&self, tenant_id: Uuid, alias: &str) -> Result<oagw_sdk::Upstream, ServiceGatewayError> {
        self.cp.resolve_upstream(tenant_id, alias).await
            .map(upstream_to_sdk).map_err(domain_err_to_sdk)
    }

    async fn resolve_route(
        &self,
        tenant_id: Uuid,
        upstream_id: Uuid,
        method: &str,
        path: &str,
    ) -> Result<oagw_sdk::Route, ServiceGatewayError> {
        self.cp.resolve_route(tenant_id, upstream_id, method, path).await
            .map(route_to_sdk).map_err(domain_err_to_sdk)
    }

    async fn proxy_request(&self, ctx: SdkProxyContext) -> Result<SdkProxyResponse, ServiceGatewayError> {
        let internal_ctx = dto::ProxyContext {
            tenant_id: ctx.tenant_id,
            method: ctx.method,
            alias: ctx.alias,
            path_suffix: ctx.path_suffix,
            query_params: ctx.query_params,
            headers: ctx.headers,
            body: ctx.body,
            instance_uri: ctx.instance_uri,
        };
        let result = self.dp.proxy_request(internal_ctx).await.map_err(domain_err_to_sdk)?;
        Ok(SdkProxyResponse {
            status: result.status,
            headers: result.headers,
            body: result.body,
            error_source: match result.error_source {
                dto::ErrorSource::Gateway => oagw_sdk::api::ErrorSource::Gateway,
                dto::ErrorSource::Upstream => oagw_sdk::api::ErrorSource::Upstream,
            },
        })
    }
}

// ---------------------------------------------------------------------------
// DomainError → ServiceGatewayError
// ---------------------------------------------------------------------------

fn domain_err_to_sdk(err: DomainError) -> ServiceGatewayError {
    match err {
        DomainError::NotFound { entity, id } => ServiceGatewayError::RouteNotFound {
            instance: format!("{entity}/{id}"),
        },
        DomainError::Conflict { detail } => ServiceGatewayError::ValidationError {
            detail,
            instance: String::new(),
        },
        DomainError::Validation { detail, instance } => ServiceGatewayError::ValidationError {
            detail,
            instance,
        },
        DomainError::UpstreamDisabled { alias } => ServiceGatewayError::UpstreamDisabled {
            detail: format!("upstream '{alias}' is disabled"),
            instance: String::new(),
        },
        DomainError::Internal { message } => ServiceGatewayError::DownstreamError {
            detail: message,
            instance: String::new(),
        },
        DomainError::MissingTargetHost { instance } => ServiceGatewayError::MissingTargetHost { instance },
        DomainError::InvalidTargetHost { instance } => ServiceGatewayError::InvalidTargetHost { instance },
        DomainError::UnknownTargetHost { detail, instance } => ServiceGatewayError::UnknownTargetHost { detail, instance },
        DomainError::AuthenticationFailed { detail, instance } => ServiceGatewayError::AuthenticationFailed { detail, instance },
        DomainError::PayloadTooLarge { detail, instance } => ServiceGatewayError::PayloadTooLarge { detail, instance },
        DomainError::RateLimitExceeded { detail, instance, retry_after_secs } => ServiceGatewayError::RateLimitExceeded { detail, instance, retry_after_secs },
        DomainError::SecretNotFound { detail, instance } => ServiceGatewayError::SecretNotFound { detail, instance },
        DomainError::DownstreamError { detail, instance } => ServiceGatewayError::DownstreamError { detail, instance },
        DomainError::ProtocolError { detail, instance } => ServiceGatewayError::ProtocolError { detail, instance },
        DomainError::ConnectionTimeout { detail, instance } => ServiceGatewayError::ConnectionTimeout { detail, instance },
        DomainError::RequestTimeout { detail, instance } => ServiceGatewayError::RequestTimeout { detail, instance },
    }
}

// ---------------------------------------------------------------------------
// SDK request → domain request conversions (using SDK getters for private fields)
// ---------------------------------------------------------------------------

fn sdk_create_upstream_to_domain(req: oagw_sdk::CreateUpstreamRequest) -> dto::CreateUpstreamRequest {
    dto::CreateUpstreamRequest {
        server: server_to_domain(req.server().clone()),
        protocol: req.protocol().to_string(),
        alias: req.alias().map(|s| s.to_string()),
        auth: req.auth().cloned().map(auth_config_to_domain),
        headers: req.headers().cloned().map(headers_config_to_domain),
        plugins: req.plugins().cloned().map(plugins_config_to_domain),
        rate_limit: req.rate_limit().cloned().map(rate_limit_config_to_domain),
        tags: req.tags().to_vec(),
        enabled: req.enabled(),
    }
}

fn sdk_update_upstream_to_domain(req: oagw_sdk::UpdateUpstreamRequest) -> dto::UpdateUpstreamRequest {
    dto::UpdateUpstreamRequest {
        server: req.server().cloned().map(server_to_domain),
        protocol: req.protocol().map(|s| s.to_string()),
        alias: req.alias().map(|s| s.to_string()),
        auth: req.auth().cloned().map(auth_config_to_domain),
        headers: req.headers().cloned().map(headers_config_to_domain),
        plugins: req.plugins().cloned().map(plugins_config_to_domain),
        rate_limit: req.rate_limit().cloned().map(rate_limit_config_to_domain),
        tags: req.tags().map(|s| s.to_vec()),
        enabled: req.enabled(),
    }
}

fn sdk_create_route_to_domain(req: oagw_sdk::CreateRouteRequest) -> dto::CreateRouteRequest {
    dto::CreateRouteRequest {
        upstream_id: req.upstream_id(),
        match_rules: match_rules_to_domain(req.match_rules().clone()),
        plugins: req.plugins().cloned().map(plugins_config_to_domain),
        rate_limit: req.rate_limit().cloned().map(rate_limit_config_to_domain),
        tags: req.tags().to_vec(),
        priority: req.priority(),
        enabled: req.enabled(),
    }
}

fn sdk_update_route_to_domain(req: oagw_sdk::UpdateRouteRequest) -> dto::UpdateRouteRequest {
    dto::UpdateRouteRequest {
        match_rules: req.match_rules().cloned().map(match_rules_to_domain),
        plugins: req.plugins().cloned().map(plugins_config_to_domain),
        rate_limit: req.rate_limit().cloned().map(rate_limit_config_to_domain),
        tags: req.tags().map(|s| s.to_vec()),
        priority: req.priority(),
        enabled: req.enabled(),
    }
}

// ---------------------------------------------------------------------------
// SDK value types → domain value types
// ---------------------------------------------------------------------------

fn sharing_mode_to_domain(v: oagw_sdk::SharingMode) -> dto::SharingMode {
    match v {
        oagw_sdk::SharingMode::Private => dto::SharingMode::Private,
        oagw_sdk::SharingMode::Inherit => dto::SharingMode::Inherit,
        oagw_sdk::SharingMode::Enforce => dto::SharingMode::Enforce,
    }
}

fn scheme_to_domain(v: oagw_sdk::Scheme) -> dto::Scheme {
    match v {
        oagw_sdk::Scheme::Http => dto::Scheme::Http,
        oagw_sdk::Scheme::Https => dto::Scheme::Https,
        oagw_sdk::Scheme::Wss => dto::Scheme::Wss,
        oagw_sdk::Scheme::Wt => dto::Scheme::Wt,
        oagw_sdk::Scheme::Grpc => dto::Scheme::Grpc,
    }
}

fn endpoint_to_domain(v: oagw_sdk::Endpoint) -> dto::Endpoint {
    dto::Endpoint { scheme: scheme_to_domain(v.scheme), host: v.host, port: v.port }
}

fn server_to_domain(v: oagw_sdk::Server) -> dto::Server {
    dto::Server { endpoints: v.endpoints.into_iter().map(endpoint_to_domain).collect() }
}

fn auth_config_to_domain(v: oagw_sdk::AuthConfig) -> dto::AuthConfig {
    dto::AuthConfig { plugin_type: v.plugin_type, sharing: sharing_mode_to_domain(v.sharing), config: v.config }
}

fn passthrough_mode_to_domain(v: oagw_sdk::PassthroughMode) -> dto::PassthroughMode {
    match v {
        oagw_sdk::PassthroughMode::None => dto::PassthroughMode::None,
        oagw_sdk::PassthroughMode::Allowlist => dto::PassthroughMode::Allowlist,
        oagw_sdk::PassthroughMode::All => dto::PassthroughMode::All,
    }
}

fn request_header_rules_to_domain(v: oagw_sdk::RequestHeaderRules) -> dto::RequestHeaderRules {
    dto::RequestHeaderRules {
        set: v.set, add: v.add, remove: v.remove,
        passthrough: passthrough_mode_to_domain(v.passthrough),
        passthrough_allowlist: v.passthrough_allowlist,
    }
}

fn response_header_rules_to_domain(v: oagw_sdk::ResponseHeaderRules) -> dto::ResponseHeaderRules {
    dto::ResponseHeaderRules { set: v.set, add: v.add, remove: v.remove }
}

fn headers_config_to_domain(v: oagw_sdk::HeadersConfig) -> dto::HeadersConfig {
    dto::HeadersConfig {
        request: v.request.map(request_header_rules_to_domain),
        response: v.response.map(response_header_rules_to_domain),
    }
}

fn window_to_domain(v: oagw_sdk::Window) -> dto::Window {
    match v {
        oagw_sdk::Window::Second => dto::Window::Second,
        oagw_sdk::Window::Minute => dto::Window::Minute,
        oagw_sdk::Window::Hour => dto::Window::Hour,
        oagw_sdk::Window::Day => dto::Window::Day,
    }
}

fn rate_limit_config_to_domain(v: oagw_sdk::RateLimitConfig) -> dto::RateLimitConfig {
    dto::RateLimitConfig {
        sharing: sharing_mode_to_domain(v.sharing),
        algorithm: match v.algorithm {
            oagw_sdk::RateLimitAlgorithm::TokenBucket => dto::RateLimitAlgorithm::TokenBucket,
            oagw_sdk::RateLimitAlgorithm::SlidingWindow => dto::RateLimitAlgorithm::SlidingWindow,
        },
        sustained: dto::SustainedRate { rate: v.sustained.rate, window: window_to_domain(v.sustained.window) },
        burst: v.burst.map(|b| dto::BurstConfig { capacity: b.capacity }),
        scope: match v.scope {
            oagw_sdk::RateLimitScope::Global => dto::RateLimitScope::Global,
            oagw_sdk::RateLimitScope::Tenant => dto::RateLimitScope::Tenant,
            oagw_sdk::RateLimitScope::User => dto::RateLimitScope::User,
            oagw_sdk::RateLimitScope::Ip => dto::RateLimitScope::Ip,
            oagw_sdk::RateLimitScope::Route => dto::RateLimitScope::Route,
        },
        strategy: match v.strategy {
            oagw_sdk::RateLimitStrategy::Reject => dto::RateLimitStrategy::Reject,
            oagw_sdk::RateLimitStrategy::Queue => dto::RateLimitStrategy::Queue,
            oagw_sdk::RateLimitStrategy::Degrade => dto::RateLimitStrategy::Degrade,
        },
        cost: v.cost,
    }
}

fn plugins_config_to_domain(v: oagw_sdk::PluginsConfig) -> dto::PluginsConfig {
    dto::PluginsConfig { sharing: sharing_mode_to_domain(v.sharing), items: v.items }
}

fn http_method_to_domain(v: oagw_sdk::HttpMethod) -> dto::HttpMethod {
    match v {
        oagw_sdk::HttpMethod::Get => dto::HttpMethod::Get,
        oagw_sdk::HttpMethod::Post => dto::HttpMethod::Post,
        oagw_sdk::HttpMethod::Put => dto::HttpMethod::Put,
        oagw_sdk::HttpMethod::Delete => dto::HttpMethod::Delete,
        oagw_sdk::HttpMethod::Patch => dto::HttpMethod::Patch,
    }
}

fn http_match_to_domain(v: oagw_sdk::HttpMatch) -> dto::HttpMatch {
    dto::HttpMatch {
        methods: v.methods.into_iter().map(http_method_to_domain).collect(),
        path: v.path,
        query_allowlist: v.query_allowlist,
        path_suffix_mode: match v.path_suffix_mode {
            oagw_sdk::PathSuffixMode::Disabled => dto::PathSuffixMode::Disabled,
            oagw_sdk::PathSuffixMode::Append => dto::PathSuffixMode::Append,
        },
    }
}

fn grpc_match_to_domain(v: oagw_sdk::GrpcMatch) -> dto::GrpcMatch {
    dto::GrpcMatch { service: v.service, method: v.method }
}

fn match_rules_to_domain(v: oagw_sdk::MatchRules) -> dto::MatchRules {
    dto::MatchRules {
        http: v.http.map(http_match_to_domain),
        grpc: v.grpc.map(grpc_match_to_domain),
    }
}

// ---------------------------------------------------------------------------
// domain value types → SDK value types
// ---------------------------------------------------------------------------

fn sharing_mode_to_sdk(v: dto::SharingMode) -> oagw_sdk::SharingMode {
    match v {
        dto::SharingMode::Private => oagw_sdk::SharingMode::Private,
        dto::SharingMode::Inherit => oagw_sdk::SharingMode::Inherit,
        dto::SharingMode::Enforce => oagw_sdk::SharingMode::Enforce,
    }
}

fn scheme_to_sdk(v: dto::Scheme) -> oagw_sdk::Scheme {
    match v {
        dto::Scheme::Http => oagw_sdk::Scheme::Http,
        dto::Scheme::Https => oagw_sdk::Scheme::Https,
        dto::Scheme::Wss => oagw_sdk::Scheme::Wss,
        dto::Scheme::Wt => oagw_sdk::Scheme::Wt,
        dto::Scheme::Grpc => oagw_sdk::Scheme::Grpc,
    }
}

fn upstream_to_sdk(u: dto::Upstream) -> oagw_sdk::Upstream {
    oagw_sdk::Upstream {
        id: u.id,
        tenant_id: u.tenant_id,
        alias: u.alias,
        server: oagw_sdk::Server {
            endpoints: u.server.endpoints.into_iter().map(|e| oagw_sdk::Endpoint {
                scheme: scheme_to_sdk(e.scheme), host: e.host, port: e.port,
            }).collect(),
        },
        protocol: u.protocol,
        enabled: u.enabled,
        auth: u.auth.map(|a| oagw_sdk::AuthConfig {
            plugin_type: a.plugin_type, sharing: sharing_mode_to_sdk(a.sharing), config: a.config,
        }),
        headers: u.headers.map(|h| oagw_sdk::HeadersConfig {
            request: h.request.map(|r| oagw_sdk::RequestHeaderRules {
                set: r.set, add: r.add, remove: r.remove,
                passthrough: match r.passthrough {
                    dto::PassthroughMode::None => oagw_sdk::PassthroughMode::None,
                    dto::PassthroughMode::Allowlist => oagw_sdk::PassthroughMode::Allowlist,
                    dto::PassthroughMode::All => oagw_sdk::PassthroughMode::All,
                },
                passthrough_allowlist: r.passthrough_allowlist,
            }),
            response: h.response.map(|r| oagw_sdk::ResponseHeaderRules {
                set: r.set, add: r.add, remove: r.remove,
            }),
        }),
        plugins: u.plugins.map(|p| oagw_sdk::PluginsConfig {
            sharing: sharing_mode_to_sdk(p.sharing), items: p.items,
        }),
        rate_limit: u.rate_limit.map(rate_limit_config_to_sdk),
        tags: u.tags,
    }
}

fn route_to_sdk(r: dto::Route) -> oagw_sdk::Route {
    oagw_sdk::Route {
        id: r.id,
        tenant_id: r.tenant_id,
        upstream_id: r.upstream_id,
        match_rules: oagw_sdk::MatchRules {
            http: r.match_rules.http.map(|h| oagw_sdk::HttpMatch {
                methods: h.methods.into_iter().map(|m| match m {
                    dto::HttpMethod::Get => oagw_sdk::HttpMethod::Get,
                    dto::HttpMethod::Post => oagw_sdk::HttpMethod::Post,
                    dto::HttpMethod::Put => oagw_sdk::HttpMethod::Put,
                    dto::HttpMethod::Delete => oagw_sdk::HttpMethod::Delete,
                    dto::HttpMethod::Patch => oagw_sdk::HttpMethod::Patch,
                }).collect(),
                path: h.path,
                query_allowlist: h.query_allowlist,
                path_suffix_mode: match h.path_suffix_mode {
                    dto::PathSuffixMode::Disabled => oagw_sdk::PathSuffixMode::Disabled,
                    dto::PathSuffixMode::Append => oagw_sdk::PathSuffixMode::Append,
                },
            }),
            grpc: r.match_rules.grpc.map(|g| oagw_sdk::GrpcMatch {
                service: g.service, method: g.method,
            }),
        },
        plugins: r.plugins.map(|p| oagw_sdk::PluginsConfig {
            sharing: sharing_mode_to_sdk(p.sharing), items: p.items,
        }),
        rate_limit: r.rate_limit.map(rate_limit_config_to_sdk),
        tags: r.tags,
        priority: r.priority,
        enabled: r.enabled,
    }
}

fn rate_limit_config_to_sdk(v: dto::RateLimitConfig) -> oagw_sdk::RateLimitConfig {
    oagw_sdk::RateLimitConfig {
        sharing: sharing_mode_to_sdk(v.sharing),
        algorithm: match v.algorithm {
            dto::RateLimitAlgorithm::TokenBucket => oagw_sdk::RateLimitAlgorithm::TokenBucket,
            dto::RateLimitAlgorithm::SlidingWindow => oagw_sdk::RateLimitAlgorithm::SlidingWindow,
        },
        sustained: oagw_sdk::SustainedRate {
            rate: v.sustained.rate,
            window: match v.sustained.window {
                dto::Window::Second => oagw_sdk::Window::Second,
                dto::Window::Minute => oagw_sdk::Window::Minute,
                dto::Window::Hour => oagw_sdk::Window::Hour,
                dto::Window::Day => oagw_sdk::Window::Day,
            },
        },
        burst: v.burst.map(|b| oagw_sdk::BurstConfig { capacity: b.capacity }),
        scope: match v.scope {
            dto::RateLimitScope::Global => oagw_sdk::RateLimitScope::Global,
            dto::RateLimitScope::Tenant => oagw_sdk::RateLimitScope::Tenant,
            dto::RateLimitScope::User => oagw_sdk::RateLimitScope::User,
            dto::RateLimitScope::Ip => oagw_sdk::RateLimitScope::Ip,
            dto::RateLimitScope::Route => oagw_sdk::RateLimitScope::Route,
        },
        strategy: match v.strategy {
            dto::RateLimitStrategy::Reject => oagw_sdk::RateLimitStrategy::Reject,
            dto::RateLimitStrategy::Queue => oagw_sdk::RateLimitStrategy::Queue,
            dto::RateLimitStrategy::Degrade => oagw_sdk::RateLimitStrategy::Degrade,
        },
        cost: v.cost,
    }
}
