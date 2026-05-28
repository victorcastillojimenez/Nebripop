use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Domain entity representing a conversation between buyer and seller
#[derive(Debug, Clone, sqlx::FromRow)]
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
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

/// Rich conversation row with joined user and listing data for API responses
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ConversationWithDetails {
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
