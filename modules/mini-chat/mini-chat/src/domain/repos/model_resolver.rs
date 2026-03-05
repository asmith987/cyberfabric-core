use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::error::DomainError;

/// Resolves and validates model IDs against the tenant's policy catalog.
///
/// If `model` is `None`, returns the default model for the tenant.
/// If `model` is `Some`, validates it is non-empty and exists in the catalog.
#[async_trait]
pub trait ModelResolver: Send + Sync {
    async fn resolve_model(
        &self,
        tenant_id: Uuid,
        model: Option<String>,
    ) -> Result<String, DomainError>;
}
