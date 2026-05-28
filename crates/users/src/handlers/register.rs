use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use validator::Validate;

use crate::adapters::user_repository::UserRepository;
use crate::dtos::{AuthResponse, RegisterDto};
use crate::errors::UserError;
use crate::usecases;

use common::errors::AppError;

/// POST /auth/register
/// Register a new user and return JWT token
pub async fn register_handler(
    State(repo): State<UserRepository>,
    State(jwt_secret): State<String>,
    Json(payload): Json<RegisterDto>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    // Validate input
    payload.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Execute use case
    let response = usecases::register_usecase::register(&repo, payload, &jwt_secret)
        .await
        .map_err(|e| match e {
            UserError::EmailAlreadyExists => {
                AppError::Conflict("El email ya está registrado".to_string())
            }
            UserError::DatabaseError(db_err) => {
                tracing::error!("Database error during register: {}", db_err);
                AppError::Internal("Error al crear usuario".to_string())
            }
            UserError::CryptoError(msg) => {
                tracing::error!("Crypto error during register: {}", msg);
                AppError::Internal("Error al procesar la contraseña".to_string())
            }
            _ => AppError::Internal("Error al registrar usuario".to_string()),
        })?;

    Ok((StatusCode::CREATED, Json(response)))
}
