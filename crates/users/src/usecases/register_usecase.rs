use crate::adapters::jwt::generate_jwt;
use crate::adapters::password::hash_password;
use crate::dtos::{AuthResponse, RegisterDto, UserDto};
use crate::errors::UserError;
use crate::models::User;
use crate::ports::UserRepositoryPort;

/// Register a new user
/// 1. Validates email is not duplicated
/// 2. Hashes password with Argon2id
/// 3. Inserts user into database
/// 4. Generates JWT token
pub async fn register(
    repo: &impl UserRepositoryPort,
    dto: RegisterDto,
    jwt_secret: &str,
) -> Result<AuthResponse, UserError> {
    // Check if email already exists
    if (repo.find_by_email(&dto.email).await?).is_some() {
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

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    use super::*;
    use crate::errors::UserError;
    use crate::models::User;
    use crate::ports::UserRepositoryPort;

    /// Mock repository that simulates an existing email for duplicate testing
    struct MockRepoWithExistingEmail {
        existing_email: String,
    }

    #[async_trait]
    impl UserRepositoryPort for MockRepoWithExistingEmail {
        async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserError> {
            if email == self.existing_email {
                Ok(Some(User {
                    id: Uuid::new_v4(),
                    email: email.to_string(),
                    password_hash: "hash".to_string(),
                    display_name: "existing".to_string(),
                    avatar_url: None,
                    phone: None,
                    role: "user".to_string(),
                    rating_avg: None,
                    total_ratings: 0,
                    last_login_at: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            } else {
                Ok(None)
            }
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, UserError> {
            Ok(None)
        }

        async fn insert(
            &self,
            _email: &str,
            _password_hash: &str,
            _display_name: &str,
        ) -> Result<User, UserError> {
            unreachable!("insert should not be called when email already exists")
        }

        async fn update_last_login(&self, _id: Uuid) -> Result<(), UserError> {
            Ok(())
        }
    }

    /// Mock repository that always succeeds on new user creation
    struct MockSuccessRepo {
        existing_emails: Arc<Mutex<Vec<String>>>,
    }

    impl MockSuccessRepo {
        fn new() -> Self {
            Self {
                existing_emails: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl UserRepositoryPort for MockSuccessRepo {
        async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserError> {
            let existing = self.existing_emails.lock().unwrap();
            if existing.contains(&email.to_string()) {
                Ok(Some(User {
                    id: Uuid::new_v4(),
                    email: email.to_string(),
                    password_hash: "hash".to_string(),
                    display_name: "existing".to_string(),
                    avatar_url: None,
                    phone: None,
                    role: "user".to_string(),
                    rating_avg: None,
                    total_ratings: 0,
                    last_login_at: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            } else {
                Ok(None)
            }
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, UserError> {
            Ok(None)
        }

        async fn insert(
            &self,
            email: &str,
            password_hash: &str,
            display_name: &str,
        ) -> Result<User, UserError> {
            let mut existing = self.existing_emails.lock().unwrap();
            existing.push(email.to_string());
            Ok(User {
                id: Uuid::new_v4(),
                email: email.to_string(),
                password_hash: password_hash.to_string(),
                display_name: display_name.to_string(),
                avatar_url: None,
                phone: None,
                role: "user".to_string(),
                rating_avg: None,
                total_ratings: 0,
                last_login_at: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        async fn update_last_login(&self, _id: Uuid) -> Result<(), UserError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_register_email_duplicate_returns_email_already_exists() {
        let repo = MockRepoWithExistingEmail {
            existing_email: "existing@test.com".to_string(),
        };
        let dto = RegisterDto {
            email: "existing@test.com".to_string(),
            password: "password123".to_string(),
            display_name: "Test User".to_string(),
        };
        let result = register(&repo, dto, "test_secret").await;
        match result {
            Err(UserError::EmailAlreadyExists) => {} // Expected
            _ => panic!("Expected EmailAlreadyExists error for duplicate email"),
        }
    }

    #[tokio::test]
    async fn test_register_new_user_success() {
        let repo = MockSuccessRepo::new();
        let dto = RegisterDto {
            email: "newuser@test.com".to_string(),
            password: "SecurePass123!".to_string(),
            display_name: "New User".to_string(),
        };
        let result = register(&repo, dto, "test_secret").await;
        assert!(result.is_ok(), "Registration should succeed for new email");
        let response = result.unwrap();
        assert!(!response.access_token.is_empty(), "Should return a JWT token");
        assert_eq!(response.expires_in, 86400, "Should expire in 24h");
        assert_eq!(response.user.email, "newuser@test.com");
        assert_eq!(response.user.display_name, "New User");
    }
}
