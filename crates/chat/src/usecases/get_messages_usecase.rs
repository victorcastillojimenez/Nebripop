use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::adapters::message_repo::MessageRepository;
use crate::dtos::MessageResponseDto;
use crate::errors::ChatError;

pub async fn execute(
    conversation_repo: &ConversationRepository,
    message_repo: &MessageRepository,
    conversation_id: Uuid,
    user_id: Uuid,
    since: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<MessageResponseDto>, ChatError> {
    // 1. Verify membership
    let is_member = conversation_repo
        .is_member(conversation_id, user_id)
        .await?;

    if !is_member {
        return Err(ChatError::NotMember(user_id));
    }

    // 2. Mark messages from the other user as read
    conversation_repo
        .mark_as_read(conversation_id, user_id)
        .await?;

    // 3. Fetch messages
    let messages = message_repo
        .find_by_conversation_id(conversation_id, since, limit)
        .await?;

    // 4. Map to DTOs
    let dtos: Vec<MessageResponseDto> = messages
        .into_iter()
        .map(|m| MessageResponseDto {
            id: m.id,
            conversation_id: m.conversation_id,
            sender_id: m.sender_id,
            content: m.content,
            is_read: m.is_read || m.sender_id == user_id,
            created_at: m.created_at,
        })
        .collect();

    Ok(dtos)
}
