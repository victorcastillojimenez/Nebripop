use sqlx::PgPool;
use uuid::Uuid;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::adapters::message_repo::MessageRepository;
use crate::dtos::{ConversationResponseDto, CreateConversationDto};
use crate::errors::ChatError;

pub async fn execute(
    conversation_repo: &ConversationRepository,
    message_repo: &MessageRepository,
    pool: &PgPool,
    buyer_id: Uuid,
    dto: CreateConversationDto,
) -> Result<ConversationResponseDto, ChatError> {
    // 1. Validate initial message length
    let initial_message = dto.initial_message.trim().to_string();
    if initial_message.is_empty() || initial_message.len() > 5000 {
        return Err(ChatError::InvalidMessage(
            "El mensaje debe tener entre 1 y 5000 caracteres".to_string(),
        ));
    }

    // 2. Check that the listing exists and get the seller_id
    let listing_row = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT seller_id FROM listings WHERE id = $1 AND status = 'active'
        "#,
    )
    .bind(dto.listing_id)
    .fetch_optional(pool)
    .await?
    .ok_or(ChatError::ListingNotFound(dto.listing_id))?;

    let seller_id = listing_row;

    // 3. Prevent chatting with yourself
    if buyer_id == seller_id {
        return Err(ChatError::CannotChatWithSelf);
    }

    // 4. Check if conversation already exists for this (listing, buyer)
    let existing = conversation_repo
        .find_by_listing_and_buyer(dto.listing_id, buyer_id)
        .await?;

    if let Some(_conv) = existing {
        return Err(ChatError::ConversationAlreadyExists(buyer_id, dto.listing_id));
    }

    // 5. Create the conversation
    let conversation = conversation_repo
        .create(dto.listing_id, buyer_id, seller_id)
        .await?;

    // 6. Create the first message
    let _message = message_repo
        .create(conversation.id, buyer_id, &initial_message)
        .await?;

    // 7. Update last_message on conversation
    conversation_repo
        .update_last_message(conversation.id, &initial_message)
        .await?;

    // 8. Fetch the enriched conversation data for the response
    let (enriched, _) = conversation_repo
        .find_by_user_id_paginated(buyer_id, 0, 1_000_000)
        .await?;

    let response = enriched
        .into_iter()
        .find(|c| c.id == conversation.id)
        .map(|c| ConversationResponseDto {
            id: c.id,
            listing_id: c.listing_id,
            listing_title: c.listing_title,
            listing_image: c.listing_image,
            other_user_id: c.other_user_id,
            other_user_name: c.other_user_name,
            other_user_avatar: c.other_user_avatar,
            last_message: c.last_message,
            last_message_at: c.last_message_at,
            unread_count: c.unread_count,
            created_at: c.created_at,
            updated_at: c.updated_at,
        })
        .ok_or_else(|| ChatError::Internal("No se pudo recuperar la conversación creada".to_string()))?;

    Ok(response)
}
