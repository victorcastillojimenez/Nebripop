use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::ChatError;
use crate::models::Conversation;
use crate::ports::{ConversationPort, ConversationWithDetails};

// ── Internal row types (infrastructure-only, with sqlx::FromRow) ──────────

/// SQL row mapping for conversations table
#[derive(Debug, Clone, sqlx::FromRow)]
struct ConversationRow {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub buyer_id: Uuid,
    pub seller_id: Uuid,
    pub last_message: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ConversationRow> for Conversation {
    fn from(row: ConversationRow) -> Self {
        Self {
            id: row.id,
            listing_id: row.listing_id,
            buyer_id: row.buyer_id,
            seller_id: row.seller_id,
            last_message: row.last_message,
            last_message_at: row.last_message_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// SQL row mapping for the enriched conversation query
#[derive(Debug, Clone, sqlx::FromRow)]
struct ConversationDetailsRow {
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

impl From<ConversationDetailsRow> for ConversationWithDetails {
    fn from(row: ConversationDetailsRow) -> Self {
        Self {
            id: row.id,
            listing_id: row.listing_id,
            listing_title: row.listing_title,
            listing_image: row.listing_image,
            other_user_id: row.other_user_id,
            other_user_name: row.other_user_name,
            other_user_avatar: row.other_user_avatar,
            last_message: row.last_message,
            last_message_at: row.last_message_at,
            unread_count: row.unread_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ── Repository implementation ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ConversationRepository {
    pool: PgPool,
}

impl ConversationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ConversationPort for ConversationRepository {
    async fn create(
        &self,
        listing_id: Uuid,
        buyer_id: Uuid,
        seller_id: Uuid,
    ) -> Result<Conversation, ChatError> {
        let row = sqlx::query_as::<_, ConversationRow>(
            r#"
            INSERT INTO conversations (id, listing_id, buyer_id, seller_id, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, $3, now(), now())
            RETURNING id, listing_id, buyer_id, seller_id, last_message, last_message_at, created_at, updated_at
            "#,
        )
        .bind(listing_id)
        .bind(buyer_id)
        .bind(seller_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Conversation, ChatError> {
        let row = sqlx::query_as::<_, ConversationRow>(
            r#"
            SELECT id, listing_id, buyer_id, seller_id, last_message, last_message_at, created_at, updated_at
            FROM conversations
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(ChatError::ConversationNotFound(id))?;

        Ok(row.into())
    }

    async fn find_by_listing_and_buyer(
        &self,
        listing_id: Uuid,
        buyer_id: Uuid,
    ) -> Result<Option<Conversation>, ChatError> {
        let row = sqlx::query_as::<_, ConversationRow>(
            r#"
            SELECT id, listing_id, buyer_id, seller_id, last_message, last_message_at, created_at, updated_at
            FROM conversations
            WHERE listing_id = $1 AND buyer_id = $2
            "#,
        )
        .bind(listing_id)
        .bind(buyer_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn find_by_user_id_paginated(
        &self,
        user_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<ConversationWithDetails>, i64), ChatError> {
        let offset = page * per_page;

        let count_row = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::bigint
            FROM conversations
            WHERE buyer_id = $1 OR seller_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, ConversationDetailsRow>(
            r#"
            SELECT
                c.id,
                c.listing_id,
                l.title AS listing_title,
                (SELECT li.image_url FROM listing_images li WHERE li.listing_id = l.id ORDER BY li.position ASC LIMIT 1) AS listing_image,
                CASE WHEN c.buyer_id = $1 THEN c.seller_id ELSE c.buyer_id END AS other_user_id,
                CASE WHEN c.buyer_id = $1 THEN us.display_name ELSE ub.display_name END AS other_user_name,
                CASE WHEN c.buyer_id = $1 THEN us.avatar_url ELSE ub.avatar_url END AS other_user_avatar,
                c.last_message,
                c.last_message_at,
                COALESCE(
                    (SELECT COUNT(*)::int FROM messages m WHERE m.conversation_id = c.id AND m.sender_id != $1 AND m.is_read = false),
                    0
                ) AS unread_count,
                c.created_at,
                c.updated_at
            FROM conversations c
            JOIN listings l ON l.id = c.listing_id
            JOIN users ub ON ub.id = c.buyer_id
            JOIN users us ON us.id = c.seller_id
            WHERE c.buyer_id = $1 OR c.seller_id = $1
            ORDER BY c.updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let conversations: Vec<ConversationWithDetails> =
            rows.into_iter().map(|r| r.into()).collect();

        Ok((conversations, count_row))
    }

    async fn update_last_message(
        &self,
        conversation_id: Uuid,
        content: &str,
    ) -> Result<(), ChatError> {
        sqlx::query(
            r#"
            UPDATE conversations
            SET last_message = $1, last_message_at = now(), updated_at = now()
            WHERE id = $2
            "#,
        )
        .bind(content)
        .bind(conversation_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn mark_as_read(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ChatError> {
        sqlx::query(
            r#"
            UPDATE messages
            SET is_read = true
            WHERE conversation_id = $1 AND sender_id != $2 AND is_read = false
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn is_member(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, ChatError> {
        let row = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            SELECT 1::bigint
            FROM conversations
            WHERE id = $1 AND (buyer_id = $2 OR seller_id = $2)
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.is_some())
    }

    async fn get_other_participant(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<Uuid, ChatError> {
        let conversation = self.find_by_id(conversation_id).await?;
        let other_id = if conversation.buyer_id == user_id {
            conversation.seller_id
        } else if conversation.seller_id == user_id {
            conversation.buyer_id
        } else {
            return Err(ChatError::NotMember(user_id));
        };
        Ok(other_id)
    }

    async fn verify_listing_seller(
        &self,
        listing_id: Uuid,
    ) -> Result<Uuid, ChatError> {
        let seller_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT seller_id FROM listings WHERE id = $1 AND status = 'active'
            "#,
        )
        .bind(listing_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(ChatError::ListingNotFound(listing_id))?;

        Ok(seller_id)
    }
}
