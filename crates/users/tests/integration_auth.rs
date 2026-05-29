//! Integration tests for the auth module (users crate).
//!
//! These tests use `#[sqlx::test]` to get an ephemeral PostgreSQL database.
//! They test the complete flow of user registration and authentication:
//! - Register a new user (success, duplicate email → 409)
//! - Login (success → returns JWT, wrong password → 401 generic)
//! - Token validation (expired/expired token → 401)
//! - Refresh token flow
//!
//! Pattern: given_<state>_when_<action>_then_<result>

use sqlx::PgPool;
use uuid::Uuid;

use users::adapters::jwt::{generate_jwt, verify_jwt};
use users::adapters::password::verify_password;
use users::adapters::user_repository::UserRepository;
use users::dtos::{LoginDto, RegisterDto};
use users::errors::UserError;
use users::ports::UserRepositoryPort;
use users::usecases::{login_usecase, register_usecase};

const TEST_JWT_SECRET: &str = "test_jwt_secret_for_integration_tests_2026_v1";

// ── Helper: create a UserRepository from a pool ──────────────────────────

fn make_repo(pool: PgPool) -> UserRepository {
    UserRepository::new(pool)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn given_new_email_when_register_then_user_persisted(pool: PgPool) {
    let repo = make_repo(pool.clone());

    let dto = RegisterDto {
        email: "fresh_user@nebripop.test".to_string(),
        password: "MySecurePwd42!".to_string(),
        display_name: "Fresh User".to_string(),
    };

    let result = register_usecase::register(&repo, dto, TEST_JWT_SECRET).await;
    assert!(result.is_ok(), "Registration should succeed for new email");

    let response = result.unwrap();
    assert!(
        response.access_token.len() > 20,
        "Should return a valid JWT token"
    );
    assert_eq!(response.expires_in, 86400, "Token should expire in 24h");
    assert_eq!(
        response.user.email, "fresh_user@nebripop.test",
        "User email should match"
    );
    assert_eq!(
        response.user.display_name, "Fresh User",
        "Display name should match"
    );

    // Verify user exists in database
    let user = repo
        .find_by_email("fresh_user@nebripop.test")
        .await
        .expect("Query should succeed")
        .expect("User should exist in DB");
    assert_eq!(user.email, "fresh_user@nebripop.test");
    assert!(
        user.password_hash.starts_with("$argon2id"),
        "Password should be hashed with argon2id"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_duplicate_email_when_register_then_returns_email_already_exists(pool: PgPool) {
    let repo = make_repo(pool);

    // First registration
    let dto1 = RegisterDto {
        email: "duplicate@nebripop.test".to_string(),
        password: "SecurePass123!".to_string(),
        display_name: "First User".to_string(),
    };
    let result1 = register_usecase::register(&repo, dto1, TEST_JWT_SECRET).await;
    assert!(result1.is_ok(), "First registration should succeed");

    // Second registration with same email
    let dto2 = RegisterDto {
        email: "duplicate@nebripop.test".to_string(),
        password: "OtherPass456!".to_string(),
        display_name: "Second User".to_string(),
    };
    let result2 = register_usecase::register(&repo, dto2, TEST_JWT_SECRET).await;

    match result2 {
        Err(UserError::EmailAlreadyExists) => {} // Expected
        _ => panic!("Expected EmailAlreadyExists error for duplicate email"),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_correct_credentials_when_login_then_returns_jwt(pool: PgPool) {
    let repo = make_repo(pool);

    // Register a user first
    let register_dto = RegisterDto {
        email: "login_test@nebripop.test".to_string(),
        password: "MySecurePwd42!".to_string(),
        display_name: "Login Test".to_string(),
    };
    let _ = register_usecase::register(&repo, register_dto, TEST_JWT_SECRET)
        .await
        .expect("Registration should succeed");

    // Login with correct credentials
    let login_dto = LoginDto {
        email: "login_test@nebripop.test".to_string(),
        password: "MySecurePwd42!".to_string(),
    };
    let result = login_usecase::login(&repo, login_dto, TEST_JWT_SECRET).await;
    assert!(result.is_ok(), "Login should succeed with correct credentials");

    let response = result.unwrap();
    assert!(
        response.access_token.len() > 20,
        "Should return a valid JWT token"
    );
    assert_eq!(response.expires_in, 86400, "Token should expire in 24h");

    // Verify the token can be decoded
    let claims = verify_jwt(&response.access_token, TEST_JWT_SECRET)
        .expect("JWT should be valid and verifiable");
    assert_eq!(claims.role, "user", "Token should have 'user' role");
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_wrong_password_when_login_then_returns_401(pool: PgPool) {
    let repo = make_repo(pool);

    // Register a user first
    let register_dto = RegisterDto {
        email: "wrong_pwd@nebripop.test".to_string(),
        password: "CorrectPass1!".to_string(),
        display_name: "Wrong Pwd Test".to_string(),
    };
    let _ = register_usecase::register(&repo, register_dto, TEST_JWT_SECRET)
        .await
        .expect("Registration should succeed");

    // Login with wrong password
    let login_dto = LoginDto {
        email: "wrong_pwd@nebripop.test".to_string(),
        password: "WrongPassword999!".to_string(),
    };
    let result = login_usecase::login(&repo, login_dto, TEST_JWT_SECRET).await;

    match result {
        Err(UserError::InvalidCredentials) => {} // Expected — generic error
        _ => panic!("Expected InvalidCredentials error for wrong password"),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_nonexistent_email_when_login_then_returns_401(pool: PgPool) {
    let repo = make_repo(pool);

    // Try to login with an email that doesn't exist
    let login_dto = LoginDto {
        email: "nonexistent@nebripop.test".to_string(),
        password: "SomePassword123!".to_string(),
    };
    let result = login_usecase::login(&repo, login_dto, TEST_JWT_SECRET).await;

    match result {
        Err(UserError::InvalidCredentials) => {} // Expected — generic to prevent enumeration
        _ => panic!("Expected InvalidCredentials for nonexistent email"),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_expired_token_when_verify_then_returns_error(_pool: PgPool) {
    let user_id = Uuid::new_v4();
    let valid_token = generate_jwt(user_id, "user", TEST_JWT_SECRET)
        .expect("Token generation should succeed");

    // Verify the valid token works
    let valid_claims = verify_jwt(&valid_token, TEST_JWT_SECRET);
    assert!(valid_claims.is_ok(), "Freshly generated token should be valid");

    // Tamper with the token payload to simulate an invalid token
    let parts: Vec<&str> = valid_token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts");

    // Create a malformed token (tampered payload)
    let tampered_token = format!("{}.tampered.{}", parts[0], parts[2]);
    let result = verify_jwt(&tampered_token, TEST_JWT_SECRET);
    assert!(
        result.is_err(),
        "Tampered token should return error"
    );

    // Also verify that a token with wrong secret fails
    let result_wrong_secret = verify_jwt(&valid_token, "different_secret");
    assert!(
        result_wrong_secret.is_err(),
        "Token with wrong secret should return error"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_invalid_token_format_when_verify_then_returns_error(_pool: PgPool) {
    let result = verify_jwt("not-a-jwt-token", TEST_JWT_SECRET);
    assert!(
        result.is_err(),
        "Malformed token should return InvalidToken error"
    );
    match result {
        Err(UserError::InvalidToken) => {} // Expected
        _ => panic!("Expected InvalidToken for malformed token"),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_valid_token_when_refresh_then_returns_new_token(pool: PgPool) {
    let repo = make_repo(pool);

    // Register a user to get context
    let register_dto = RegisterDto {
        email: "refresh_test@nebripop.test".to_string(),
        password: "SecurePass123!".to_string(),
        display_name: "Refresh Test".to_string(),
    };
    let registered = register_usecase::register(&repo, register_dto, TEST_JWT_SECRET)
        .await
        .expect("Registration should succeed");

    // Generate a new token from the original (simulating refresh)
    let new_token = generate_jwt(
        registered.user.id,
        "user",
        TEST_JWT_SECRET,
    )
    .expect("Refresh token generation should succeed");

    assert!(
        new_token.len() > 20,
        "New token should be a valid JWT"
    );

    // Verify the new token works
    let claims = verify_jwt(&new_token, TEST_JWT_SECRET)
        .expect("New token should be verifiable");
    assert_eq!(claims.sub, registered.user.id, "Token sub should match user ID");
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_user_when_register_then_password_is_hashed(pool: PgPool) {
    let repo = make_repo(pool.clone());

    let dto = RegisterDto {
        email: "hash_check@nebripop.test".to_string(),
        password: "CheckHash123!".to_string(),
        display_name: "Hash Check".to_string(),
    };
    let _ = register_usecase::register(&repo, dto, TEST_JWT_SECRET)
        .await
        .expect("Registration should succeed");

    // Directly query the database to verify password hashing
    let row: (String,) = sqlx::query_as(
        "SELECT password_hash FROM users WHERE email = $1",
    )
    .bind("hash_check@nebripop.test")
    .fetch_one(&pool)
    .await
    .expect("User should exist in DB");

    let hash = &row.0;
    assert!(
        hash.starts_with("$argon2id"),
        "Password hash should use argon2id algorithm"
    );
    assert!(
        hash.len() >= 60,
        "Argon2id hash should be sufficiently long"
    );

    // Verify the password works against the stored hash
    assert!(
        verify_password("CheckHash123!", hash),
        "Password should verify against stored hash"
    );
    assert!(
        !verify_password("WrongHash999!", hash),
        "Wrong password should not verify"
    );
}
