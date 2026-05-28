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
    .map_err(|_| UserError::InvalidToken)?;

    Ok(token_data.claims)
}

/// Generate a JWT token with a specific expiration offset (for testing purposes)
#[cfg(test)]
fn generate_jwt_with_expiry(
    user_id: Uuid,
    role: &str,
    jwt_secret: &str,
    exp_offset_seconds: i64,
) -> Result<String, UserError> {
    let now = Utc::now();
    let expiration = now
        .checked_add_signed(Duration::seconds(exp_offset_seconds))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify_valid_token() {
        let user_id = Uuid::new_v4();
        let secret = "test_secret_key_for_jwt_testing_1234567890";

        let token = generate_jwt(user_id, "user", secret)
            .expect("Token generation should succeed");

        let claims = verify_jwt(&token, secret)
            .expect("Token verification should succeed");

        assert_eq!(claims.sub, user_id, "User ID should match");
        assert_eq!(claims.role, "user", "Role should match");
        assert!(claims.iat > 0, "iat should be set");
        assert!(claims.exp > claims.iat, "exp should be after iat");
    }

    #[test]
    fn test_expired_token_returns_invalid_token() {
        let user_id = Uuid::new_v4();
        let secret = "test_secret_key_for_jwt_testing_1234567890";

        // Generate token with -7200 second expiry (already expired 2 hours ago)
        let token = generate_jwt_with_expiry(user_id, "user", secret, -7200)
            .expect("Token generation should succeed");

        let result = verify_jwt(&token, secret);
        assert!(result.is_err(), "Expired token should return error");
        match result {
            Err(UserError::InvalidToken) => {} // Expected
            _ => panic!("Expected InvalidToken error for expired token"),
        }
    }

    #[test]
    fn test_verify_invalid_token_format() {
        let secret = "test_secret_key_for_jwt_testing_1234567890";
        let result = verify_jwt("invalid.token.here", secret);
        assert!(result.is_err(), "Invalid token should return error");
        match result {
            Err(UserError::InvalidToken) => {} // Expected
            _ => panic!("Expected InvalidToken error for malformed token"),
        }
    }

    #[test]
    fn test_verify_with_wrong_secret() {
        let user_id = Uuid::new_v4();
        let secret = "test_secret_key_for_jwt_testing_1234567890";
        let wrong_secret = "different_secret_key_that_should_not_match_12345";

        let token = generate_jwt(user_id, "user", secret)
            .expect("Token generation should succeed");

        let result = verify_jwt(&token, wrong_secret);
        assert!(result.is_err(), "Token with wrong secret should return error");
        match result {
            Err(UserError::InvalidToken) => {} // Expected
            _ => panic!("Expected InvalidToken error for wrong secret"),
        }
    }
}
