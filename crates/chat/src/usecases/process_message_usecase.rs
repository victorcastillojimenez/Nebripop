use axum::extract::ws::Message as WsMessage;
use uuid::Uuid;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::adapters::message_repo::MessageRepository;
use crate::connections::ActiveConnections;
use crate::dtos::{MessageResponseDto, SendMessageDto};
use crate::errors::ChatError;

/// Process a text message received via WebSocket
/// 1. Deserialize and validate
/// 2. Persist to PostgreSQL
/// 3. Broadcast to recipient in real-time if connected
pub async fn process_received_text(
    text: String,
    sender_id: Uuid,
    conversation_id: Uuid,
    conversation_repo: &ConversationRepository,
    message_repo: &MessageRepository,
    active_connections: &ActiveConnections,
) -> Result<(), ChatError> {
    // 1. Deserialize payload
    let payload: SendMessageDto = serde_json::from_str(&text).map_err(|_| {
        ChatError::InvalidMessage("Formato JSON inválido".to_string())
    })?;

    // 2. Validate content
    let content = payload.content.trim().to_string();
    if content.is_empty() || content.len() > 5000 {
        return Err(ChatError::InvalidMessage(
            "Mensaje debe tener entre 1 y 5000 caracteres".to_string(),
        ));
    }

    // 3. Persist to PostgreSQL FIRST (guarantee persistence before broadcast)
    let msg = message_repo
        .create(conversation_id, sender_id, &content)
        .await?;

    conversation_repo
        .update_last_message(conversation_id, &content)
        .await?;

    // 4. Identify recipient
    let recipient_id = conversation_repo
        .get_other_participant(conversation_id, sender_id)
        .await?;

    // 5. Build response DTO
    let response = MessageResponseDto {
        id: msg.id,
        conversation_id: msg.conversation_id,
        sender_id: msg.sender_id,
        content: msg.content,
        is_read: false,
        created_at: msg.created_at,
    };

    let json = serde_json::to_string(&response).map_err(|e| {
        ChatError::Internal(format!("Error serializando mensaje: {}", e))
    })?;

    // 6. Send to recipient if connected
    if let Some(tx) = active_connections
        .map
        .get(&(conversation_id, recipient_id))
    {
        if tx.send(WsMessage::Text(json.clone())).is_err() {
            tracing::warn!(
                "Failed to send WS message to recipient {} in conversation {}",
                recipient_id,
                conversation_id
            );
        }
    }

    // 7. Echo back to sender for confirmation
    if let Some(tx) = active_connections
        .map
        .get(&(conversation_id, sender_id))
    {
        if tx.send(WsMessage::Text(json)).is_err() {
            tracing::warn!(
                "Failed to echo WS message to sender {} in conversation {}",
                sender_id,
                conversation_id
            );
        }
    }

    Ok(())
}
