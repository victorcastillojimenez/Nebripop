use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::UserError;

/// JWT Claims — only contains sub (user_id), role, exp, iat
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

/// Generate a JWT HS256 token with 24h expiration
pub fn generate_jwt(user_id: Uuid, role: &str, jwt_secret: &str) -> Result<String, UserError> {
    let now = Utc::now();
    let expiration = now
        .checked_add_signed(Duration::hours(24))
        .ok_or_else(|| UserError::CryptoError("Error al calcular expiración".to_string()))?;

    let claims = Claims {
        sub: user_id,
        role: role.to_string(),
        exp: expiration.timestamp(),
        iat: now.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| UserError::CryptoError(format!("Error al generar token: {}", e)))?;

    Ok(token)
}

/// Verify a JWT token and return the claims
pub fn verify_jwt(token: &str, jwt_secret: &str) -> Result<Claims, UserError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                UserError::InvalidToken
            }
            _ => UserError::InvalidToken,
        }
    })?;

    Ok(token_data.claims)
}
