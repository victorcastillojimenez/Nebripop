use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::ChatError;
use crate::models::Message;
use crate::ports::MessagePort;

// ── Internal row type (infrastructure-only, with sqlx::FromRow) ──────────

/// SQL row mapping for messages table
#[derive(Debug, Clone, sqlx::FromRow)]
struct MessageRow {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

impl From<MessageRow> for Message {
    fn from(row: MessageRow) -> Self {
        Self {
            id: row.id,
            conversation_id: row.conversation_id,
            sender_id: row.sender_id,
            content: row.content,
            is_read: row.is_read,
            created_at: row.created_at,
        }
    }
}

// ── Repository implementation ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MessageRepository {
    pool: PgPool,
}

impl MessageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MessagePort for MessageRepository {
    async fn create(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        content: &str,
    ) -> Result<Message, ChatError> {
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

        let row = sqlx::query_as::<_, MessageRow>(
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

        Ok(row.into())
    }

    async fn find_by_conversation_id(
        &self,
        conversation_id: Uuid,
        since: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<Message>, ChatError> {
        let limit = limit.clamp(1, 200);

        let rows = if let Some(since) = since {
            sqlx::query_as::<_, MessageRow>(
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
            sqlx::query_as::<_, MessageRow>(
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

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn count_unread(
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
