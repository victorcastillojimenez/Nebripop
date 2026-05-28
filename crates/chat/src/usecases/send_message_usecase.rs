use uuid::Uuid;

use crate::dtos::{MessageResponseDto, SendMessageDto};
use crate::errors::ChatError;
use crate::ports::{BroadcastPort, ConversationPort, MessagePort};

/// Validate and normalize message content.
fn validate_content(raw: &str) -> Result<String, ChatError> {
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() || trimmed.len() > 5000 {
        return Err(ChatError::InvalidMessage(
            "El mensaje debe tener entre 1 y 5000 caracteres".to_string(),
        ));
    }
    Ok(trimmed)
}

/// Persist a new message and update the conversation metadata.
async fn persist_new_message(
    conversation_port: &impl ConversationPort,
    message_port: &impl MessagePort,
    conversation_id: Uuid,
    sender_id: Uuid,
    content: &str,
) -> Result<crate::models::Message, ChatError> {
    let new_message = message_port
        .create(conversation_id, sender_id, content)
        .await?;

    conversation_port
        .update_last_message(conversation_id, content)
        .await?;

    Ok(new_message)
}

/// Send the message DTO to both participants via WebSocket.
async fn broadcast_message_dto(
    broadcaster: &impl BroadcastPort,
    conversation_id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    response: &MessageResponseDto,
) {
    if let Ok(json) = serde_json::to_string(response) {
        broadcaster
            .send_to_user(conversation_id, recipient_id, &json)
            .await;
        broadcaster
            .send_to_user(conversation_id, sender_id, &json)
            .await;
    }
}

/// Send a message via HTTP (fallback when WebSocket is unavailable).
/// 1. Validate content
/// 2. Verify membership
/// 3. Persist the message
/// 4. Broadcast to participants in real-time if connected
pub async fn execute(
    conversation_port: &impl ConversationPort,
    message_port: &impl MessagePort,
    broadcaster: &impl BroadcastPort,
    conversation_id: Uuid,
    sender_id: Uuid,
    dto: SendMessageDto,
) -> Result<MessageResponseDto, ChatError> {
    // 1. Validate content
    let content = validate_content(&dto.content)?;

    // 2. Verify membership
    let is_member = conversation_port
        .is_member(conversation_id, sender_id)
        .await?;

    if !is_member {
        return Err(ChatError::NotMember(sender_id));
    }

    // 3. Persist the message (before broadcast)
    let new_message = persist_new_message(
        conversation_port,
        message_port,
        conversation_id,
        sender_id,
        &content,
    )
    .await?;

    // 4. Identify recipient
    let recipient_id = conversation_port
        .get_other_participant(conversation_id, sender_id)
        .await?;

    // 5. Build response DTO
    let response = MessageResponseDto {
        id: new_message.id,
        conversation_id: new_message.conversation_id,
        sender_id: new_message.sender_id,
        content: new_message.content,
        is_read: false,
        created_at: new_message.created_at,
    };

    // 6. Broadcast to participants via WebSocket
    broadcast_message_dto(
        broadcaster,
        conversation_id,
        sender_id,
        recipient_id,
        &response,
    )
    .await;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_content_rejects_empty() {
        let result = validate_content("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_content_rejects_whitespace() {
        let result = validate_content("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_content_rejects_oversized() {
        let long_string = "x".repeat(5001);
        let result = validate_content(&long_string);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_content_accepts_valid() {
        let result = validate_content("Hello, world!");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");
    }
}
