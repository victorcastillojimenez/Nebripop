use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::dtos::MessageResponseDto;
use crate::errors::ChatError;
use crate::ports::{ConversationPort, MessagePort};

/// Map a domain Message to a MessageResponseDto.
/// Messages sent by the current user are always considered "read".
fn map_to_dto(
    message: crate::models::Message,
    current_user_id: Uuid,
) -> MessageResponseDto {
    MessageResponseDto {
        id: message.id,
        conversation_id: message.conversation_id,
        sender_id: message.sender_id,
        content: message.content,
        is_read: message.is_read || message.sender_id == current_user_id,
        created_at: message.created_at,
    }
}

/// Get messages from a conversation with optional HTTP polling support.
/// 1. Verify membership
/// 2. Mark messages from the other user as read
/// 3. Fetch and return messages
pub async fn execute(
    conversation_port: &impl ConversationPort,
    message_port: &impl MessagePort,
    conversation_id: Uuid,
    current_user_id: Uuid,
    since: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<MessageResponseDto>, ChatError> {
    // 1. Verify membership
    let is_member = conversation_port
        .is_member(conversation_id, current_user_id)
        .await?;

    if !is_member {
        return Err(ChatError::NotMember(current_user_id));
    }

    // 2. Mark messages from the other user as read
    conversation_port
        .mark_as_read(conversation_id, current_user_id)
        .await?;

    // 3. Fetch messages
    let messages = message_port
        .find_by_conversation_id(conversation_id, since, limit)
        .await?;

    // 4. Map to DTOs
    let dtos: Vec<MessageResponseDto> = messages
        .into_iter()
        .map(|message| map_to_dto(message, current_user_id))
        .collect();

    Ok(dtos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_to_dto_marks_own_messages_as_read() {
        let user_id = Uuid::new_v4();
        let message = crate::models::Message {
            id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            sender_id: user_id,
            content: "Hello".to_string(),
            is_read: false,
            created_at: Utc::now(),
        };

        let dto = map_to_dto(message, user_id);
        assert!(dto.is_read);
        assert_eq!(dto.sender_id, user_id);
    }

    #[test]
    fn test_map_to_dto_other_user_unread_message() {
        let current_user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();
        let message = crate::models::Message {
            id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            sender_id: other_user_id,
            content: "Hello".to_string(),
            is_read: false,
            created_at: Utc::now(),
        };

        let dto = map_to_dto(message, current_user_id);
        assert!(!dto.is_read);
        assert_eq!(dto.sender_id, other_user_id);
    }
}
