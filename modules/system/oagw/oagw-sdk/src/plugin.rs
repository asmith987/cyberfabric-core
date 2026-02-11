use http::HeaderMap;

// ---------------------------------------------------------------------------
// Plugin errors
// ---------------------------------------------------------------------------

/// Errors returned by plugin execution.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("secret not found: {0}")]
    SecretNotFound(String),
    #[error("authentication failed: {0}")]
    AuthFailed(String),
    #[error("request rejected: {0}")]
    Rejected(String),
    #[error("plugin error: {0}")]
    Internal(String),
}

// ---------------------------------------------------------------------------
// Auth plugin
// ---------------------------------------------------------------------------

/// Context passed to authentication plugins.
pub struct AuthContext {
    /// Outbound request headers (mutable — plugin can inject credentials).
    pub headers: HeaderMap,
    /// Plugin-specific configuration from the upstream auth config.
    pub config: serde_json::Value,
}

/// Trait for authentication plugins that inject credentials into outbound requests.
#[async_trait::async_trait]
pub trait AuthPlugin: Send + Sync {
    /// Modify the outbound request headers to inject authentication credentials.
    ///
    /// # Errors
    /// Returns `PluginError` if credential resolution or injection fails.
    async fn authenticate(&self, ctx: &mut AuthContext) -> Result<(), PluginError>;
}
