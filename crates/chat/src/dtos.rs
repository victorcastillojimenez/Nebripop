use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Input DTOs ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConversationDto {
    pub listing_id: Uuid,
    pub initial_message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageDto {
    #[serde(deserialize_with = "deserialize_non_empty")]
    pub content: String,
}

fn deserialize_non_empty<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let s = String::deserialize(d)?;
    if s.trim().is_empty() {
        return Err(serde::de::Error::custom("content must not be empty"));
    }
    Ok(s)
}

#[derive(Debug, Deserialize)]
pub struct PollingQuery {
    pub since: Option<DateTime<Utc>>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct WsQueryParams {
    pub token: String,
}

// ── Output DTOs ────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationResponseDto {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub listing_title: String,
    pub listing_image: Option<String>,
    pub other_user_id: Uuid,
    pub other_user_name: String,
    pub other_user_avatar: Option<String>,
    pub last_message: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub unread_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationListResponseDto {
    pub conversations: Vec<ConversationResponseDto>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageResponseDto {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}
