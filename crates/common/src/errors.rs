use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Serialize)]
pub struct ErrorResponseBody {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Error de validación: {0}")]
    ValidationError(String),

    #[error("No autorizado: {0}")]
    Unauthorized(String),

    #[error("Acceso denegado: {0}")]
    Forbidden(String),

    #[error("Recurso no encontrado: {0}")]
    NotFound(String),

    #[error("Conflicto: {0}")]
    Conflict(String),

    #[error("Petición incorrecta: {0}")]
    BadRequest(String),

    #[error("Error interno del servidor")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),
            AppError::Internal(msg) => {
                tracing::error!("Internal server error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_SERVER_ERROR",
                    "Ha ocurrido un error interno del servidor".to_string(),
                )
            }
        };

        let body = Json(ErrorResponseBody {
            error: error_code.to_string(),
            message,
            details: None,
        });

        (status, body).into_response()
    }
}
