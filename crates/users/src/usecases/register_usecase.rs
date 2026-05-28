use crate::adapters::jwt::generate_jwt;
use crate::adapters::password::hash_password;
use crate::adapters::user_repository::UserRepository;
use crate::dtos::{AuthResponse, RegisterDto, UserDto};
use crate::errors::UserError;
use crate::models::User;

/// Register a new user
/// 1. Validates email is not duplicated
/// 2. Hashes password with Argon2id
/// 3. Inserts user into database
/// 4. Generates JWT token
pub async fn register(
    repo: &UserRepository,
    dto: RegisterDto,
    jwt_secret: &str,
) -> Result<AuthResponse, UserError> {
    // Check if email already exists
    if let Some(_) = repo.find_by_email(&dto.email).await? {
        return Err(UserError::EmailAlreadyExists);
    }

    // Hash password with Argon2id (OWASP parameters)
    let password_hash = hash_password(&dto.password)?;

    // Insert user into database
    let user: User = repo.insert(&dto.email, &password_hash, &dto.display_name).await?;

    // Generate JWT token
    let access_token = generate_jwt(user.id, &user.role, jwt_secret)?;

    Ok(AuthResponse {
        access_token,
        expires_in: 86400,
        user: UserDto::from(user),
    })
}
