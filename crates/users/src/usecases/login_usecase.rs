use crate::adapters::jwt::generate_jwt;
use crate::adapters::password::verify_password;
use crate::dtos::{LoginDto, TokenResponse};
use crate::errors::UserError;
use crate::ports::UserRepositoryPort;

/// Authenticate a user with email and password
/// 1. Look up user by email
/// 2. Verify password against stored hash
/// 3. Update last_login_at
/// 4. Generate JWT token
///
/// Returns generic "Credenciales incorrectas" for both wrong email and wrong password
pub async fn login(
    repo: &impl UserRepositoryPort,
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

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use uuid::Uuid;

    use super::*;
    use crate::adapters::password::hash_password;
    use crate::errors::UserError;
    use crate::models::User;
    use crate::ports::UserRepositoryPort;

    /// Mock repository that always returns a specific user by email
    struct MockRepo {
        user: Option<User>,
    }

    impl MockRepo {
        fn with_user(email: &str, password: &str, display_name: &str) -> Self {
            let hash = hash_password(password).expect("Hashing should succeed");
            Self {
                user: Some(User {
                    id: Uuid::new_v4(),
                    email: email.to_string(),
                    password_hash: hash,
                    display_name: display_name.to_string(),
                    avatar_url: None,
                    phone: None,
                    role: "user".to_string(),
                    rating_avg: None,
                    total_ratings: 0,
                    last_login_at: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }),
            }
        }

        fn empty() -> Self {
            Self { user: None }
        }
    }

    #[async_trait]
    impl UserRepositoryPort for MockRepo {
        async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserError> {
            match &self.user {
                Some(user) if user.email == email => Ok(Some(user.clone())),
                _ => Ok(None),
            }
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, UserError> {
            Ok(self.user.clone())
        }

        async fn insert(
            &self,
            _email: &str,
            _password_hash: &str,
            _display_name: &str,
        ) -> Result<User, UserError> {
            unreachable!("insert should not be called in login tests")
        }

        async fn update_last_login(&self, _id: Uuid) -> Result<(), UserError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_login_with_nonexistent_email_returns_invalid_credentials() {
        let repo = MockRepo::empty();
        let dto = LoginDto {
            email: "nonexistent@test.com".to_string(),
            password: "password123".to_string(),
        };
        let result = login(&repo, dto, "test_secret").await;
        match result {
            Err(UserError::InvalidCredentials) => {} // Expected
            _ => panic!("Expected InvalidCredentials for nonexistent email"),
        }
    }

    #[tokio::test]
    async fn test_login_with_wrong_password_returns_invalid_credentials() {
        let repo = MockRepo::with_user("user@test.com", "CorrectPass1", "Test User");
        let dto = LoginDto {
            email: "user@test.com".to_string(),
            password: "WrongPassword".to_string(),
        };
        let result = login(&repo, dto, "test_secret").await;
        match result {
            Err(UserError::InvalidCredentials) => {} // Expected
            _ => panic!("Expected InvalidCredentials for wrong password"),
        }
    }

    #[tokio::test]
    async fn test_login_with_correct_credentials_returns_token() {
        let repo = MockRepo::with_user("valid@test.com", "MySecurePwd1", "Valid User");
        let dto = LoginDto {
            email: "valid@test.com".to_string(),
            password: "MySecurePwd1".to_string(),
        };
        let result = login(&repo, dto, "test_secret").await;
        assert!(result.is_ok(), "Login should succeed with correct credentials");
        let response = result.unwrap();
        assert!(!response.access_token.is_empty(), "Should return a JWT token");
        assert_eq!(response.expires_in, 86400, "Should expire in 24h");
    }
}
