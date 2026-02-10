use std::time::Duration;

use crate::error::ClientError;
use crate::remote_proxy::RemoteProxyClient;
use crate::request::Request;
use crate::response::Response;

/// Main OAGW client with deployment-agnostic API
pub struct OagwClient {
    inner: OagwClientImpl,
}

enum OagwClientImpl {
    RemoteProxy(RemoteProxyClient),
    // Future: SharedProcess(SharedProcessClient),
}

/// Configuration for OagwClient
#[derive(Debug, Clone)]
pub struct OagwClientConfig {
    pub mode: ClientMode,
    pub default_timeout: Duration,
}

/// Client deployment mode
#[derive(Debug, Clone)]
pub enum ClientMode {
    /// OAGW in separate process - HTTP calls to proxy endpoint
    RemoteProxy {
        base_url: String,
        auth_token: String,
        timeout: Duration,
    },
    // Future:
    // SharedProcess { control_plane: Arc<dyn ControlPlaneService> },
}

impl OagwClientConfig {
    /// Create configuration for remote OAGW mode
    pub fn remote(base_url: String, auth_token: String) -> Self {
        Self {
            mode: ClientMode::RemoteProxy {
                base_url,
                auth_token,
                timeout: Duration::from_secs(30),
            },
            default_timeout: Duration::from_secs(30),
        }
    }

    /// Set custom timeout for remote mode
    pub fn with_timeout(mut self, new_timeout: Duration) -> Self {
        match self.mode {
            ClientMode::RemoteProxy {
                ref mut timeout, ..
            } => {
                *timeout = new_timeout;
            }
        }
        self.default_timeout = new_timeout;
        self
    }

    /// Create configuration from environment variables
    ///
    /// Expects:
    /// - `OAGW_BASE_URL`: Base URL for OAGW service (default: "https://oagw.internal.cf")
    /// - `OAGW_AUTH_TOKEN`: Authentication token (required)
    pub fn from_env() -> Result<Self, ClientError> {
        let base_url = std::env::var("OAGW_BASE_URL")
            .unwrap_or_else(|_| "https://oagw.internal.cf".to_string());
        let auth_token = std::env::var("OAGW_AUTH_TOKEN")
            .map_err(|_| ClientError::BuildError("OAGW_AUTH_TOKEN not set".into()))?;

        Ok(Self::remote(base_url, auth_token))
    }
}

impl OagwClient {
    /// Create client from configuration
    pub fn from_config(config: OagwClientConfig) -> Result<Self, ClientError> {
        let inner = match config.mode {
            ClientMode::RemoteProxy {
                base_url,
                auth_token,
                timeout,
            } => {
                OagwClientImpl::RemoteProxy(RemoteProxyClient::new(
                    base_url, auth_token, timeout,
                )?)
            }
        };
        Ok(Self { inner })
    }

    /// Execute HTTP request through OAGW
    ///
    /// # Arguments
    /// * `alias` - The external service alias (e.g., "openai", "github")
    /// * `request` - The HTTP request to execute
    ///
    /// # Returns
    /// A `Response` that can be consumed in various ways (buffered, streaming, SSE)
    pub async fn execute(&self, alias: &str, request: Request) -> Result<Response, ClientError> {
        match &self.inner {
            OagwClientImpl::RemoteProxy(c) => c.execute(alias, request).await,
        }
    }

    /// Blocking version for sync contexts (e.g., build scripts)
    ///
    /// This method will use the current tokio runtime if available,
    /// or create a temporary runtime if called from a non-async context.
    ///
    /// # Arguments
    /// * `alias` - The external service alias (e.g., "openai", "github")
    /// * `request` - The HTTP request to execute
    ///
    /// # Returns
    /// A `Response` that can be consumed in various ways
    pub fn execute_blocking(&self, alias: &str, request: Request) -> Result<Response, ClientError> {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle.block_on(self.execute(alias, request)),
            Err(_) => {
                // No runtime exists - create a temporary one
                tokio::runtime::Runtime::new()?.block_on(self.execute(alias, request))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_remote() {
        let config = OagwClientConfig::remote(
            "http://localhost:8080".to_string(),
            "test-token".to_string(),
        );
        assert!(matches!(config.mode, ClientMode::RemoteProxy { .. }));
        assert_eq!(config.default_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_config_with_timeout() {
        let config = OagwClientConfig::remote(
            "http://localhost:8080".to_string(),
            "test-token".to_string(),
        )
        .with_timeout(Duration::from_secs(60));

        let ClientMode::RemoteProxy { timeout, .. } = config.mode;
        assert_eq!(timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_client_creation() {
        let config = OagwClientConfig::remote(
            "http://localhost:8080".to_string(),
            "test-token".to_string(),
        );
        let client = OagwClient::from_config(config);
        assert!(client.is_ok());
    }
}
