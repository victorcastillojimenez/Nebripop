use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::ChatError;
use crate::models::{Conversation, ConversationWithDetails};

#[derive(Debug, Clone)]
pub struct ConversationRepository {
    pool: PgPool,
}

impl ConversationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new conversation
    pub async fn create(
        &self,
        listing_id: Uuid,
        buyer_id: Uuid,
        seller_id: Uuid,
    ) -> Result<Conversation, ChatError> {
        let conversation = sqlx::query_as::<_, Conversation>(
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

        Ok(conversation)
    }

    /// Find a conversation by its ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Conversation, ChatError> {
        let conversation = sqlx::query_as::<_, Conversation>(
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

        Ok(conversation)
    }

    /// Check if a conversation already exists for a (listing, buyer) pair
    pub async fn find_by_listing_and_buyer(
        &self,
        listing_id: Uuid,
        buyer_id: Uuid,
    ) -> Result<Option<Conversation>, ChatError> {
        let conversation = sqlx::query_as::<_, Conversation>(
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

        Ok(conversation)
    }

    /// Find conversations by user ID (either as buyer or seller), ordered by last activity
    pub async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Conversation>, ChatError> {
        let conversations = sqlx::query_as::<_, Conversation>(
            r#"
            SELECT id, listing_id, buyer_id, seller_id, last_message, last_message_at, created_at, updated_at
            FROM conversations
            WHERE buyer_id = $1 OR seller_id = $1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(conversations)
    }

    /// Find conversations paginated with enriched data (user names, listing titles, unread counts)
    #[allow(clippy::type_complexity)]
    pub async fn find_by_user_id_paginated(
        &self,
        user_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<ConversationWithDetails>, i64), ChatError> {
        let offset = page * per_page;

        // Count total conversations for the user
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

        // Fetch paginated conversations with joined data
        let conversations = sqlx::query_as::<_, ConversationWithDetails>(
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

        Ok((conversations, count_row))
    }

    /// Update the last_message and last_message_at fields on a conversation
    pub async fn update_last_message(
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

    /// Mark messages as read for a given conversation and user
    /// Marks all messages from the other user as read
    pub async fn mark_as_read(
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

    /// Check if a user is a member (buyer or seller) of a conversation
    pub async fn is_member(
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

    /// Get the other participant in a conversation
    pub async fn get_other_participant(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<Uuid, ChatError> {
        let conv = self.find_by_id(conversation_id).await?;
        let other_id = if conv.buyer_id == user_id {
            conv.seller_id
        } else if conv.seller_id == user_id {
            conv.buyer_id
        } else {
            return Err(ChatError::NotMember(user_id));
        };
        Ok(other_id)
    }
}
