use axum::extract::ws::Message as WsMessage;
use uuid::Uuid;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::adapters::message_repo::MessageRepository;
use crate::connections::ActiveConnections;
use crate::dtos::{MessageResponseDto, SendMessageDto};
use crate::errors::ChatError;

pub async fn execute(
    conversation_repo: &ConversationRepository,
    message_repo: &MessageRepository,
    active_connections: &ActiveConnections,
    conversation_id: Uuid,
    sender_id: Uuid,
    dto: SendMessageDto,
) -> Result<MessageResponseDto, ChatError> {
    // 1. Validate content
    let content = dto.content.trim().to_string();
    if content.is_empty() || content.len() > 5000 {
        return Err(ChatError::InvalidMessage(
            "El mensaje debe tener entre 1 y 5000 caracteres".to_string(),
        ));
    }

    // 2. Verify membership
    let is_member = conversation_repo
        .is_member(conversation_id, sender_id)
        .await?;

    if !is_member {
        return Err(ChatError::NotMember(sender_id));
    }

    // 3. Persist the message (before broadcast)
    let message = message_repo
        .create(conversation_id, sender_id, &content)
        .await?;

    // 4. Update last_message on conversation
    conversation_repo
        .update_last_message(conversation_id, &content)
        .await?;

    // 5. Identify recipient and broadcast in real-time if connected
    let recipient_id = conversation_repo
        .get_other_participant(conversation_id, sender_id)
        .await?;

    let response = MessageResponseDto {
        id: message.id,
        conversation_id: message.conversation_id,
        sender_id: message.sender_id,
        content: message.content,
        is_read: false,
        created_at: message.created_at,
    };

    // Send via WebSocket if recipient is connected
    if let Some(tx) = active_connections
        .map
        .get(&(conversation_id, recipient_id))
    {
        if let Ok(json) = serde_json::to_string(&response) {
            if tx.send(WsMessage::Text(json)).is_err() {
                tracing::warn!(
                    "Failed to send WS message to user {} in conversation {}",
                    recipient_id,
                    conversation_id
                );
            }
        }
    }

    // Also send back to the sender so they can see their own message in real-time
    if let Some(tx) = active_connections
        .map
        .get(&(conversation_id, sender_id))
    {
        if let Ok(json) = serde_json::to_string(&response) {
            if tx.send(WsMessage::Text(json)).is_err() {
                tracing::warn!(
                    "Failed to send WS message back to sender {} in conversation {}",
                    sender_id,
                    conversation_id
                );
            }
        }
    }

    Ok(response)
}
