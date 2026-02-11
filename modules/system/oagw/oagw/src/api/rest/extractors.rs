use axum::extract::FromRequestParts;
use http::request::Parts;
use crate::domain::error::DomainError;
use crate::domain::gts_helpers;
use crate::domain::dto::ListQuery;
use uuid::Uuid;

use crate::api::rest::error::error_response;

/// Extracts the tenant ID from the `X-Tenant-Id` header.
pub struct TenantId(pub Uuid);

impl<S: Send + Sync> FromRequestParts<S> for TenantId {
    type Rejection = axum::response::Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let instance = parts.uri.path().to_string();
        let header = parts.headers.get("x-tenant-id").ok_or_else(|| {
            error_response(DomainError::Validation {
                detail: "missing X-Tenant-Id header".into(),
                instance: instance.clone(),
            })
        })?;
        let uuid_str = header.to_str().map_err(|_| {
            error_response(DomainError::Validation {
                detail: "invalid X-Tenant-Id header".into(),
                instance: instance.clone(),
            })
        })?;
        let uuid = Uuid::parse_str(uuid_str).map_err(|_| {
            error_response(DomainError::Validation {
                detail: format!("invalid X-Tenant-Id: '{uuid_str}' is not a valid UUID"),
                instance,
            })
        })?;
        Ok(TenantId(uuid))
    }
}

/// Parse a GTS identifier to extract the UUID.
///
/// # Errors
/// Returns an error response if the GTS string is invalid.
#[allow(clippy::result_large_err)]
pub fn parse_gts_id(gts_str: &str, _instance: &str) -> Result<Uuid, axum::response::Response> {
    let (_, uuid) = gts_helpers::parse_resource_gts(gts_str).map_err(error_response)?;
    Ok(uuid)
}

/// Pagination query parameters.
#[derive(Debug, serde::Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_top")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_top() -> u32 {
    50
}

impl PaginationQuery {
    pub fn to_list_query(&self) -> ListQuery {
        ListQuery {
            top: self.limit.min(100),
            skip: self.offset,
        }
    }
}
