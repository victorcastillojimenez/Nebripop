use thiserror::Error;
use uuid::Uuid;

use common::errors::AppError;

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("Conversación con ID {0} no encontrada")]
    ConversationNotFound(Uuid),

    #[error("El usuario {0} no pertenece a esta conversación")]
    NotMember(Uuid),

    #[error("Anuncio con ID {0} no encontrado")]
    ListingNotFound(Uuid),

    #[error("Ya existe una conversación entre el comprador {0} y el anuncio {1}")]
    ConversationAlreadyExists(Uuid, Uuid),

    #[error("No puedes chatear contigo mismo")]
    CannotChatWithSelf,

    #[error("Mensaje inválido: {0}")]
    InvalidMessage(String),

    #[error("Error de base de datos: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Error interno: {0}")]
    Internal(String),
}

impl From<ChatError> for AppError {
    fn from(err: ChatError) -> Self {
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
}
