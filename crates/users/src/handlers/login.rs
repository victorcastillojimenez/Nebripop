use axum::extract::State;
use axum::Json;
use validator::Validate;

use crate::adapters::user_repository::UserRepository;
use crate::dtos::{LoginDto, TokenResponse};
use crate::errors::UserError;
use crate::usecases;

use common::errors::AppError;

/// POST /auth/login
/// Authenticate user with email and password
/// Returns 401 with generic message for invalid credentials
pub async fn login_handler(
    State(repo): State<UserRepository>,
    State(jwt_secret): State<String>,
    Json(payload): Json<LoginDto>,
) -> Result<Json<TokenResponse>, AppError> {
    // Validate input
    payload.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Execute use case
    let response = usecases::login_usecase::login(&repo, payload, &jwt_secret)
        .await
        .map_err(|e| match e {
            UserError::InvalidCredentials => {
                AppError::Unauthorized("Credenciales incorrectas".to_string())
            }
            UserError::DatabaseError(db_err) => {
                tracing::error!("Database error during login: {}", db_err);
                AppError::Internal("Error interno del servidor".to_string())
            }
            UserError::CryptoError(msg) => {
                tracing::error!("Crypto error during login: {}", msg);
                AppError::Internal("Error interno del servidor".to_string())
            }
            _ => AppError::Unauthorized("Credenciales incorrectas".to_string()),
        })?;

    Ok(Json(response))
}
