use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use common::errors::AppError;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::adapters::message_repo::MessageRepository;
use crate::auth::ChatUser;
use crate::connections::ActiveConnections;
use crate::dtos::{MessageResponseDto, SendMessageDto};
use crate::errors::ChatError;
use crate::usecases;

/// POST /chat/:id/messages — Send a message via HTTP (fallback when WebSocket is unavailable)
pub async fn handle(
    State(conversation_repo): State<ConversationRepository>,
    State(message_repo): State<MessageRepository>,
    State(active_connections): State<ActiveConnections>,
    Path(conversation_id): Path<Uuid>,
    user: ChatUser,
    Json(dto): Json<SendMessageDto>,
) -> Result<(StatusCode, Json<MessageResponseDto>), AppError> {
    let result = usecases::send_message_usecase::execute(
        &conversation_repo,
        &message_repo,
        &active_connections,
        conversation_id,
        user.id,
        dto,
    )
    .await
    .map_err(|e| match e {
        ChatError::ConversationNotFound(id) => {
            AppError::NotFound(format!("Conversación {} no encontrada", id))
        }
        ChatError::NotMember(_) => {
            AppError::Forbidden("No tienes acceso a esta conversación".to_string())
        }
        ChatError::InvalidMessage(msg) => AppError::BadRequest(msg),
        ChatError::Database(db_err) => {
            tracing::error!("Database error: {:?}", db_err);
            AppError::Internal("Error de base de datos".to_string())
        }
        _ => {
            tracing::error!("Error sending message: {:?}", e);
            AppError::Internal(e.to_string())
        }
    })?;

    Ok((StatusCode::CREATED, Json(result)))
}
