//! HTTP DTOs (serde/utoipa) — REST-only request and response types.
//!
//! All REST DTOs live here; SDK `models.rs` stays transport-agnostic.
//! Provide `From` conversions between SDK models and DTOs in this file.
//!
//! Stream event types live in `domain::stream_events`; SSE wire conversion
//! and ordering enforcement live in `api::rest::sse`.

use crate::domain::models::ChatDetail;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

// ════════════════════════════════════════════════════════════════════════════
// Chat CRUD DTOs
// ════════════════════════════════════════════════════════════════════════════

/// Request DTO for creating a new chat.
#[derive(Debug, Clone)]
#[modkit_macros::api_dto(request)]
pub struct CreateChatReq {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Request DTO for updating a chat title.
#[derive(Debug, Clone)]
#[modkit_macros::api_dto(request)]
pub struct UpdateChatReq {
    pub title: String,
}

/// Response DTO for chat details.
#[derive(Debug, Clone)]
#[modkit_macros::api_dto(response)]
pub struct ChatDetailDto {
    pub id: Uuid,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub is_temporary: bool,
    pub message_count: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<ChatDetail> for ChatDetailDto {
    fn from(d: ChatDetail) -> Self {
        Self {
            id: d.id,
            model: d.model,
            title: d.title,
            is_temporary: d.is_temporary,
            message_count: d.message_count,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Streaming request DTOs
// ════════════════════════════════════════════════════════════════════════════

/// Request body for `POST /v1/chats/{id}/messages/stream`.
#[derive(Debug, Clone, serde::Deserialize, ToSchema)]
pub struct StreamMessageRequest {
    /// Message content (must be non-empty).
    pub content: String,
    /// Client-generated idempotency key (UUID v4). Optional in P1.
    #[serde(default)]
    pub request_id: Option<uuid::Uuid>,
    /// Attachment IDs to include.
    #[serde(default)]
    pub attachment_ids: Vec<uuid::Uuid>,
    /// Web search configuration.
    #[serde(default)]
    pub web_search: Option<WebSearchConfig>,
}

impl modkit::api::api_dto::RequestApiDto for StreamMessageRequest {}

/// Web search toggle.
#[derive(Debug, Clone, serde::Deserialize, ToSchema)]
pub struct WebSearchConfig {
    pub enabled: bool,
}
