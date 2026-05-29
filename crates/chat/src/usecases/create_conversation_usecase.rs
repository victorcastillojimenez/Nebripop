use uuid::Uuid;

use crate::dtos::{ConversationResponseDto, CreateConversationDto};
use crate::errors::ChatError;
use crate::ports::{ConversationPort, ConversationWithDetails, MessagePort};

/// Validate the initial message content (1-5000 chars, non-empty).
fn validate_initial_message(raw: &str) -> Result<String, ChatError> {
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() || trimmed.len() > 5000 {
        return Err(ChatError::InvalidMessage(
            "El mensaje debe tener entre 1 y 5000 caracteres".to_string(),
        ));
    }
    Ok(trimmed)
}

/// Ensure the buyer is not trying to chat with themselves.
fn ensure_not_self_chat(buyer_id: Uuid, seller_id: Uuid) -> Result<(), ChatError> {
    if buyer_id == seller_id {
        return Err(ChatError::CannotChatWithSelf);
    }
    Ok(())
}

/// Verify the listing exists and return the seller ID.
async fn verify_listing_exists(
    conversation_port: &impl ConversationPort,
    listing_id: Uuid,
) -> Result<Uuid, ChatError> {
    conversation_port.verify_listing_seller(listing_id).await
}
/// Create the conversation and its first message, then update last_message.
async fn create_conversation_with_first_message(
    conversation_port: &impl ConversationPort,
    message_port: &impl MessagePort,
    listing_id: Uuid,
    buyer_id: Uuid,
    seller_id: Uuid,
    initial_message: &str,
) -> Result<Uuid, ChatError> {
    let conversation = conversation_port
        .create(listing_id, buyer_id, seller_id)
        .await?;

    message_port
        .create(conversation.id, buyer_id, initial_message)
        .await?;

    conversation_port
        .update_last_message(conversation.id, initial_message)
        .await?;

    Ok(conversation.id)
}

/// Fetch the enriched conversation data and map it to a response DTO.
async fn fetch_enriched_response(
    conversation_port: &impl ConversationPort,
    conversation_id: Uuid,
    buyer_id: Uuid,
) -> Result<ConversationResponseDto, ChatError> {
    let (enriched, _) = conversation_port
        .find_by_user_id_paginated(buyer_id, 0, 1_000_000)
        .await?;

    enriched
        .into_iter()
        .find(|c| c.id == conversation_id)
        .map(map_to_response)
        .ok_or_else(|| {
            ChatError::Internal("No se pudo recuperar la conversación creada".to_string())
        })
}

/// Map a ConversationWithDetails to its API response DTO.
fn map_to_response(conversation: ConversationWithDetails) -> ConversationResponseDto {
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

/// Create a new conversation linked to a listing with an initial message.
pub async fn execute(
    conversation_port: &impl ConversationPort,
    message_port: &impl MessagePort,
    buyer_id: Uuid,
    dto: CreateConversationDto,
) -> Result<ConversationResponseDto, ChatError> {
    let initial_message = validate_initial_message(&dto.initial_message)?;
    let seller_id = verify_listing_exists(conversation_port, dto.listing_id).await?;

    ensure_not_self_chat(buyer_id, seller_id)?;

    // If conversation already exists for this listing + buyer, return it instead of creating a new one
    let existing = conversation_port
        .find_by_listing_and_buyer(dto.listing_id, buyer_id)
        .await?;

    if let Some(existing_conv) = existing {
        return fetch_enriched_response(conversation_port, existing_conv.id, buyer_id).await;
    }

    let conversation_id = create_conversation_with_first_message(
        conversation_port,
        message_port,
        dto.listing_id,
        buyer_id,
        seller_id,
        &initial_message,
    )
    .await?;

    fetch_enriched_response(conversation_port, conversation_id, buyer_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_initial_message_rejects_empty() {
        let result = validate_initial_message("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_initial_message_rejects_whitespace() {
        let result = validate_initial_message("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_initial_message_rejects_oversized() {
        let long_string = "x".repeat(5001);
        let result = validate_initial_message(&long_string);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_initial_message_accepts_valid() {
        let result = validate_initial_message("Hello, I'm interested!");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, I'm interested!");
    }

    #[test]
    fn test_ensure_not_self_chat_rejects_same_user() {
        let user_id = Uuid::new_v4();
        let result = ensure_not_self_chat(user_id, user_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChatError::CannotChatWithSelf));
    }

    #[test]
    fn test_ensure_not_self_chat_accepts_different_users() {
        let buyer = Uuid::new_v4();
        let seller = Uuid::new_v4();
        assert!(ensure_not_self_chat(buyer, seller).is_ok());
    }

    #[test]
    fn test_map_to_response_produces_correct_dto() {
        let now = chrono::Utc::now();
        let conversation = ConversationWithDetails {
            id: Uuid::new_v4(),
            listing_id: Uuid::new_v4(),
            listing_title: "My Listing".to_string(),
            listing_image: None,
            other_user_id: Uuid::new_v4(),
            other_user_name: "John".to_string(),
            other_user_avatar: None,
            last_message: Some("Hi".to_string()),
            last_message_at: Some(now),
            unread_count: 1,
            created_at: now,
            updated_at: now,
        };

        let dto = map_to_response(conversation);
        assert_eq!(dto.listing_title, "My Listing");
        assert_eq!(dto.other_user_name, "John");
    }
}
