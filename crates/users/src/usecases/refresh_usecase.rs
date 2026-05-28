use crate::adapters::jwt::{generate_jwt, verify_jwt};
use crate::dtos::TokenResponse;
use crate::errors::UserError;

/// Refresh a JWT token
/// 1. Verify the current token is valid
/// 2. Generate a new token with fresh expiration
pub async fn refresh(
    token: &str,
    jwt_secret: &str,
) -> Result<TokenResponse, UserError> {
    // Verify current token
    let claims = verify_jwt(token, jwt_secret)?;

    // Generate new token
    let access_token = generate_jwt(claims.sub, &claims.role, jwt_secret)?;

    Ok(TokenResponse {
        access_token,
        expires_in: 86400,
    })
}
