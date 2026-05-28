use uuid::Uuid;

use crate::dtos::{MessageResponseDto, SendMessageDto};
use crate::errors::ChatError;
use crate::ports::{BroadcastPort, ConversationPort, MessagePort};

/// Validate and normalize message content (max 5000 chars, non-empty).
fn validate_content(raw: &str) -> Result<String, ChatError> {
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() || trimmed.len() > 5000 {
        return Err(ChatError::InvalidMessage(
            "Mensaje debe tener entre 1 y 5000 caracteres".to_string(),
        ));
    }
    Ok(trimmed)
}

/// Persist a message and update the conversation's last_message pointer.
async fn persist_and_update(
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

/// Broadcast the message JSON to both participants via WebSocket.
async fn broadcast_to_participants(
    broadcaster: &impl BroadcastPort,
    conversation_id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    json_payload: &str,
) {
    broadcaster
        .send_to_user(conversation_id, recipient_id, json_payload)
        .await;
    broadcaster
        .send_to_user(conversation_id, sender_id, json_payload)
        .await;
}

/// Process a text message received via WebSocket.
/// 1. Deserialize and validate
/// 2. Persist to PostgreSQL FIRST (guarantee persistence before broadcast)
/// 3. Broadcast to recipient and sender in real-time if connected
pub async fn process_received_text(
    text: String,
    sender_id: Uuid,
    conversation_id: Uuid,
    conversation_port: &impl ConversationPort,
    message_port: &impl MessagePort,
    broadcaster: &impl BroadcastPort,
) -> Result<(), ChatError> {
    // 1. Deserialize payload
    let payload: SendMessageDto = serde_json::from_str(&text).map_err(|_| {
        ChatError::InvalidMessage("Formato JSON inválido".to_string())
    })?;

    // 2. Validate content
    let content = validate_content(&payload.content)?;

    // 3. Persist to PostgreSQL FIRST (guarantee persistence before broadcast)
    let new_message = persist_and_update(
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

    let json = serde_json::to_string(&response).map_err(|error| {
        ChatError::Internal(format!("Error serializando mensaje: {}", error))
    })?;

    // 6. Broadcast to recipient and echo to sender
    broadcast_to_participants(
        broadcaster,
        conversation_id,
        sender_id,
        recipient_id,
        &json,
    )
    .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_content_accepts_valid_message() {
        let result = validate_content("Hello, how are you?");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, how are you?");
    }

    #[test]
    fn test_validate_content_rejects_empty_message() {
        let result = validate_content("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChatError::InvalidMessage(_)));
    }

    #[test]
    fn test_validate_content_rejects_whitespace_only() {
        let result = validate_content("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_content_rejects_oversized_message() {
        let long_string = "a".repeat(5001);
        let result = validate_content(&long_string);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_content_accepts_boundary_length() {
        let long_string = "a".repeat(5000);
        let result = validate_content(&long_string);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deserialize_invalid_json_fails() {
        let result = serde_json::from_str::<SendMessageDto>("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_valid_json_succeeds() {
        let result = serde_json::from_str::<SendMessageDto>(r#"{"content": "Hello"}"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "Hello");
    }
}
