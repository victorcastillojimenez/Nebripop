use crate::adapters::jwt::generate_jwt;
use crate::adapters::password::verify_password;
use crate::adapters::user_repository::UserRepository;
use crate::dtos::{LoginDto, TokenResponse};
use crate::errors::UserError;

/// Authenticate a user with email and password
/// 1. Look up user by email
/// 2. Verify password against stored hash
/// 3. Update last_login_at
/// 4. Generate JWT token
///
/// Returns generic "Credenciales incorrectas" for both wrong email and wrong password
pub async fn login(
    repo: &UserRepository,
    dto: LoginDto,
    jwt_secret: &str,
) -> Result<TokenResponse, UserError> {
    // Find user by email — generic error to avoid account enumeration
    let user = repo
        .find_by_email(&dto.email)
        .await?
        .ok_or(UserError::InvalidCredentials)?;

    // Verify password — generic error for wrong password too
    if !verify_password(&dto.password, &user.password_hash) {
        return Err(UserError::InvalidCredentials);
    }

    // Update last login timestamp
    repo.update_last_login(user.id).await?;

    // Generate JWT token
    let access_token = generate_jwt(user.id, &user.role, jwt_secret)?;

    Ok(TokenResponse {
        access_token,
        expires_in: 86400,
    })
}
