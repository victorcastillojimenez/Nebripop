use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use common::errors::AppError;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::adapters::message_repo::MessageRepository;
use crate::auth::ChatUser;
use crate::dtos::{ConversationResponseDto, CreateConversationDto};
use crate::errors::ChatError;
use crate::usecases;

/// POST /chat — Create a new conversation linked to a listing
pub async fn handle(
    State(conversation_repo): State<ConversationRepository>,
    State(message_repo): State<MessageRepository>,
    State(pool): State<sqlx::PgPool>,
    user: ChatUser,
    Json(dto): Json<CreateConversationDto>,
) -> Result<(StatusCode, Json<ConversationResponseDto>), AppError> {
    let result = usecases::create_conversation_usecase::execute(
        &conversation_repo,
        &message_repo,
        &pool,
        user.id,
        dto,
    )
    .await
    .map_err(|e| map_chat_error(e))?;

    Ok((StatusCode::CREATED, Json(result)))
}

fn map_chat_error(err: ChatError) -> AppError {
    match err {
        ChatError::ConversationNotFound(id) => {
            AppError::NotFound(format!("Conversación {} no encontrada", id))
        }
        ChatError::NotMember(_) => {
            AppError::Forbidden("No tienes acceso a esta conversación".to_string())
        }
        ChatError::ListingNotFound(id) => {
            AppError::NotFound(format!("Anuncio {} no encontrado", id))
        }
        ChatError::ConversationAlreadyExists(buyer_id, listing_id) => {
            AppError::Conflict(format!(
                "Ya existe una conversación entre el comprador {} y el anuncio {}",
                buyer_id, listing_id
            ))
        }
        ChatError::CannotChatWithSelf => {
            AppError::BadRequest("No puedes chatear contigo mismo".to_string())
        }
        ChatError::InvalidMessage(msg) => AppError::BadRequest(msg),
        ChatError::Database(db_err) => {
            tracing::error!("Database error: {:?}", db_err);
            AppError::Internal("Error de base de datos".to_string())
        }
        ChatError::Internal(msg) => {
            tracing::error!("Internal error: {}", msg);
            AppError::Internal("Error interno del servidor".to_string())
        }
    }
}
