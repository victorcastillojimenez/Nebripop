use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Domain entity representing a conversation between buyer and seller
/// This is a pure domain model — no infrastructure annotations.
#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub buyer_id: Uuid,
    pub seller_id: Uuid,
    pub last_message: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Domain entity representing a single message in a conversation
/// This is a pure domain model — no infrastructure annotations.
#[derive(Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}
