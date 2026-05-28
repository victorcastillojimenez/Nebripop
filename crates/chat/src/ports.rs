use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::errors::ChatError;
use crate::models::{Conversation, Message};

/// Enriched conversation data for API responses (read-model, crosses aggregate boundaries)
#[derive(Debug, Clone)]
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

/// Repository trait for conversation persistence (domain port)
#[async_trait]
pub trait ConversationPort: Send + Sync {
    async fn create(
        &self,
        listing_id: Uuid,
        buyer_id: Uuid,
        seller_id: Uuid,
    ) -> Result<Conversation, ChatError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Conversation, ChatError>;

    async fn find_by_listing_and_buyer(
        &self,
        listing_id: Uuid,
        buyer_id: Uuid,
    ) -> Result<Option<Conversation>, ChatError>;

    async fn find_by_user_id_paginated(
        &self,
        user_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<ConversationWithDetails>, i64), ChatError>;

    async fn update_last_message(
        &self,
        conversation_id: Uuid,
        content: &str,
    ) -> Result<(), ChatError>;

    async fn mark_as_read(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ChatError>;

    async fn is_member(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, ChatError>;

    async fn get_other_participant(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<Uuid, ChatError>;

    async fn verify_listing_seller(
        &self,
        listing_id: Uuid,
    ) -> Result<Uuid, ChatError>;
}

/// Repository trait for message persistence (domain port)
#[async_trait]
pub trait MessagePort: Send + Sync {
    async fn create(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        content: &str,
    ) -> Result<Message, ChatError>;

    async fn find_by_conversation_id(
        &self,
        conversation_id: Uuid,
        since: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<Message>, ChatError>;

    async fn count_unread(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<i32, ChatError>;
}

/// Trait for broadcasting messages to connected WebSocket clients (domain port)
#[async_trait]
pub trait BroadcastPort: Send + Sync {
    /// Send a message to a specific user in a conversation via WebSocket
    async fn send_to_user(
        &self,
        conversation_id: Uuid,
        recipient_id: Uuid,
        json_payload: &str,
    );
}
