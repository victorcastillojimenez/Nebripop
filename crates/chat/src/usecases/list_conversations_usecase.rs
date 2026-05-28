use uuid::Uuid;

use crate::dtos::{ConversationListResponseDto, ConversationResponseDto};
use crate::errors::ChatError;
use crate::ports::{ConversationPort, ConversationWithDetails};

/// Map an enriched conversation to its API response DTO.
fn map_to_response_dto(conversation: ConversationWithDetails) -> ConversationResponseDto {
    ConversationResponseDto {
        id: conversation.id,
        listing_id: conversation.listing_id,
        listing_title: conversation.listing_title,
        listing_image: conversation.listing_image,
        other_user_id: conversation.other_user_id,
        other_user_name: conversation.other_user_name,
        other_user_avatar: conversation.other_user_avatar,
        last_message: conversation.last_message,
        last_message_at: conversation.last_message_at,
        unread_count: conversation.unread_count,
        created_at: conversation.created_at,
        updated_at: conversation.updated_at,
    }
}

/// List all conversations for the authenticated user with pagination.
pub async fn execute(
    conversation_port: &impl ConversationPort,
    current_user_id: Uuid,
    page: i64,
    per_page: i64,
) -> Result<ConversationListResponseDto, ChatError> {
    let (conversations, total) = conversation_port
        .find_by_user_id_paginated(current_user_id, page, per_page)
        .await?;

    let dtos: Vec<ConversationResponseDto> = conversations
        .into_iter()
        .map(map_to_response_dto)
        .collect();

    Ok(ConversationListResponseDto {
        conversations: dtos,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_conversation() -> ConversationWithDetails {
        let now = chrono::Utc::now();
        ConversationWithDetails {
            id: Uuid::new_v4(),
            listing_id: Uuid::new_v4(),
            listing_title: "Test Listing".to_string(),
            listing_image: None,
            other_user_id: Uuid::new_v4(),
            other_user_name: "Other User".to_string(),
            other_user_avatar: None,
            last_message: Some("Hello".to_string()),
            last_message_at: Some(now),
            unread_count: 2,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_map_to_response_dto_maps_all_fields() {
        let conversation = sample_conversation();
        let dto = map_to_response_dto(conversation);

        assert_eq!(dto.listing_title, "Test Listing");
        assert_eq!(dto.other_user_name, "Other User");
        assert_eq!(dto.unread_count, 2);
        assert_eq!(dto.last_message.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_empty_conversations_list_returns_empty() {
        let result = ConversationListResponseDto {
            conversations: vec![],
            total: 0,
        };
        assert!(result.conversations.is_empty());
        assert_eq!(result.total, 0);
    }
}
