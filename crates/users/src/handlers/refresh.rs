use axum::extract::State;
use axum::Json;

use crate::adapters::jwt::verify_jwt;
use crate::dtos::TokenResponse;
use crate::usecases;

use common::errors::AppError;

/// POST /auth/refresh
/// Generate a new JWT token from an existing valid token
/// Requires Authorization: Bearer <token> header
pub async fn refresh_handler(
    State(jwt_secret): State<String>,
    headers: axum::http::HeaderMap,
) -> Result<Json<TokenResponse>, AppError> {
    // Extract Bearer token from Authorization header
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Token no proporcionado".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized("Formato de token inválido".to_string()));
    }

    let token = &auth_header[7..];

    // First verify the token is valid (to catch expired tokens explicitly)
    let _claims = verify_jwt(token, &jwt_secret).map_err(|e| match e {
        crate::errors::UserError::InvalidToken => {
            AppError::Unauthorized("El token de sesión ha expirado".to_string())
        }
        _ => AppError::Unauthorized("Token inválido".to_string()),
    })?;

    // Execute refresh use case
    let response = usecases::refresh_usecase::refresh(token, &jwt_secret)
        .await
        .map_err(|e| match e {
            crate::errors::UserError::InvalidToken => {
                AppError::Unauthorized("El token de sesión ha expirado".to_string())
            }
            _ => AppError::Unauthorized("Token inválido".to_string()),
        })?;

    Ok(Json(response))
}
