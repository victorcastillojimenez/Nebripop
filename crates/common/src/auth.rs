use axum::async_trait;
use axum::extract::FromRef;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Claims {
    pub sub: Uuid,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

/// Authenticated user extracted from JWT token.
/// Reusable by all crates without circular dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub role: String,
}

/// FromRequestParts implementation that works with any state S
/// as long as S: FromRef<S> for String (jwt_secret).
///
/// This lives in `common` so all crates can use AuthUser as an
/// Axum extractor without depending on the `api` crate.
#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    String: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Cabecera Authorization no encontrada".to_string()))?;

        // Check Bearer prefix
        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized(
                "El token debe usar el formato Bearer".to_string(),
            ));
        }

        let token = &auth_header[7..];
        let jwt_secret = String::from_ref(state);

        // Decode and validate JWT
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|err| {
            if let jsonwebtoken::errors::ErrorKind::ExpiredSignature = err.kind() {
                AppError::Unauthorized("El token de sesión ha expirado".to_string())
            } else {
                AppError::Unauthorized("Token de sesión inválido".to_string())
            }
        })?;

        Ok(AuthUser {
            id: token_data.claims.sub,
            role: token_data.claims.role,
        })
    }
}
