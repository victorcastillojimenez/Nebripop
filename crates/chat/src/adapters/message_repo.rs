use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::ChatError;
use crate::models::Message;

#[derive(Debug, Clone)]
pub struct MessageRepository {
    pool: PgPool,
}

impl MessageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new message in a conversation
    /// Validates content length (1-5000 characters)
    pub async fn create(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        content: &str,
    ) -> Result<Message, ChatError> {
        // Validate content
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(ChatError::InvalidMessage(
                "El mensaje no puede estar vacío".to_string(),
            ));
        }
        if trimmed.len() > 5000 {
            return Err(ChatError::InvalidMessage(
                "El mensaje no puede exceder los 5000 caracteres".to_string(),
            ));
        }

        let message = sqlx::query_as::<_, Message>(
            r#"
            INSERT INTO messages (id, conversation_id, sender_id, content, is_read, created_at)
            VALUES (gen_random_uuid(), $1, $2, $3, false, now())
            RETURNING id, conversation_id, sender_id, content, is_read, created_at
            "#,
        )
        .bind(conversation_id)
        .bind(sender_id)
        .bind(trimmed)
        .fetch_one(&self.pool)
        .await?;

        Ok(message)
    }

    /// Find messages in a conversation, optionally filtered by creation time
    /// Ordered by created_at ASC, limited by the given limit (default 50, max 200)
    pub async fn find_by_conversation_id(
        &self,
        conversation_id: Uuid,
        since: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<Message>, ChatError> {
        let limit = limit.min(200).max(1);

        let messages = if let Some(since) = since {
            sqlx::query_as::<_, Message>(
                r#"
                SELECT id, conversation_id, sender_id, content, is_read, created_at
                FROM messages
                WHERE conversation_id = $1 AND created_at > $2
                ORDER BY created_at ASC
                LIMIT $3
                "#,
            )
            .bind(conversation_id)
            .bind(since)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Message>(
                r#"
                SELECT id, conversation_id, sender_id, content, is_read, created_at
                FROM messages
                WHERE conversation_id = $1
                ORDER BY created_at ASC
                LIMIT $2
                "#,
            )
            .bind(conversation_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(messages)
    }

    /// Count unread messages for a user in a conversation
    pub async fn count_unread(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<i32, ChatError> {
        let count = sqlx::query_scalar::<_, Option<i32>>(
            r#"
            SELECT COUNT(*)::int
            FROM messages
            WHERE conversation_id = $1 AND sender_id != $2 AND is_read = false
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(count)
    }
}
