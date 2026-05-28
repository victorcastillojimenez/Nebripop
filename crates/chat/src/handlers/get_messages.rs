use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use common::errors::AppError;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::adapters::message_repo::MessageRepository;
use crate::auth::ChatUser;
use crate::dtos::{MessageResponseDto, PollingQuery};
use crate::errors::ChatError;
use crate::usecases;

/// GET /chat/:id/messages — Get messages with optional HTTP polling (since parameter)
pub async fn handle(
    State(conversation_repo): State<ConversationRepository>,
    State(message_repo): State<MessageRepository>,
    Path(conversation_id): Path<Uuid>,
    user: ChatUser,
    Query(polling): Query<PollingQuery>,
) -> Result<Json<Vec<MessageResponseDto>>, AppError> {
    let result = usecases::get_messages_usecase::execute(
        &conversation_repo,
        &message_repo,
        conversation_id,
        user.id,
        polling.since,
        polling.limit,
    )
    .await
    .map_err(|e| match e {
        ChatError::ConversationNotFound(id) => {
            AppError::NotFound(format!("Conversación {} no encontrada", id))
        }
        ChatError::NotMember(_) => {
            AppError::Forbidden("No tienes acceso a esta conversación".to_string())
        }
        ChatError::Database(db_err) => {
            tracing::error!("Database error: {:?}", db_err);
            AppError::Internal("Error de base de datos".to_string())
        }
        _ => {
            tracing::error!("Error getting messages: {:?}", e);
            AppError::Internal(e.to_string())
        }
    })?;

    Ok(Json(result))
}
