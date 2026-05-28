use uuid::Uuid;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::dtos::{ConversationListResponseDto, ConversationResponseDto};
use crate::errors::ChatError;

pub async fn execute(
    repo: &ConversationRepository,
    user_id: Uuid,
    page: i64,
    per_page: i64,
) -> Result<ConversationListResponseDto, ChatError> {
    let (conversations, total) = repo
        .find_by_user_id_paginated(user_id, page, per_page)
        .await?;

    let dtos: Vec<ConversationResponseDto> = conversations
        .into_iter()
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
        .collect();

    Ok(ConversationListResponseDto {
        conversations: dtos,
        total,
    })
}
