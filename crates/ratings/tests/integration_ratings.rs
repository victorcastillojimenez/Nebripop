//! Integration tests for the ratings module.
//!
//! These tests exercise the real RatingRepository against an ephemeral
//! PostgreSQL database managed by `#[sqlx::test]`.
//!
//! Covers:
//! - Creating a valid rating (happy path)
//! - Creating a duplicate rating (expects 409 Conflict / AlreadyRated)

use sqlx::PgPool;
use uuid::Uuid;

use ratings::adapters::rating_repository::RatingRepository;
use ratings::errors::RatingError;
use ratings::ports::RatingPort;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// Creates a minimal user row for FK satisfaction.
async fn fixture_user(pool: &PgPool, seed: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, display_name)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(format!("{}_@test.com", seed))
    .bind("$argon2id$v=19$m=19456,t=2,p=1$testhash")
    .bind(seed)
    .execute(pool)
    .await
    .expect("fixture: failed to create user");
    id
}

/// Creates a minimal listing row.
async fn fixture_listing(pool: &PgPool, seller_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO listings (id, seller_id, title, description, price, category, condition, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'active')",
    )
    .bind(id)
    .bind(seller_id)
    .bind("Producto de prueba")
    .bind("Descripción del producto")
    .bind(rust_decimal::Decimal::new(2500, 2)) // 25.00
    .bind("test")
    .bind("used")
    .execute(pool)
    .await
    .expect("fixture: failed to create listing");
    id
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations/")]
async fn given_valid_score_when_create_rating_then_persisted(pool: PgPool) {
    // Arrange
    let rater_id = fixture_user(&pool, "rater").await;
    let rated_id = fixture_user(&pool, "rated").await;
    let listing_id = fixture_listing(&pool, rated_id).await;

    let repo = RatingRepository::new(pool.clone());

    // Act
    let result = repo
        .insert_rating(
            Uuid::new_v4(),
            listing_id,
            rater_id,
            rated_id,
            5, // score
            Some("Excelente producto, muy recomendado"),
        )
        .await;

    // Assert
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    let rating = result.unwrap();
    assert_eq!(rating.score, 5);
    assert_eq!(rating.listing_id, listing_id);
    assert_eq!(rating.rater_id, rater_id);
    assert_eq!(rating.rated_id, rated_id);
    assert_eq!(
        rating.comment.as_deref(),
        Some("Excelente producto, muy recomendado")
    );

    // Verify via direct DB query
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ratings WHERE listing_id = $1 AND rater_id = $2")
            .bind(listing_id)
            .bind(rater_id)
            .fetch_one(&pool)
            .await
            .expect("DB query failed");
    assert_eq!(count.0, 1, "Rating should be persisted in DB");
}

#[sqlx::test(migrations = "../../migrations/")]
async fn given_existing_rating_when_create_duplicate_then_returns_409(pool: PgPool) {
    // Arrange
    let rater_id = fixture_user(&pool, "rater_dup").await;
    let rated_id = fixture_user(&pool, "rated_dup").await;
    let listing_id = fixture_listing(&pool, rated_id).await;

    let repo = RatingRepository::new(pool.clone());

    // First insert — should succeed
    let first = repo
        .insert_rating(
            Uuid::new_v4(),
            listing_id,
            rater_id,
            rated_id,
            4,
            Some("Buena experiencia"),
        )
        .await;
    assert!(first.is_ok(), "First rating should succeed, got {:?}", first);

    // Act — second insert with same (listing_id, rater_id) → duplicate
    let second = repo
        .insert_rating(
            Uuid::new_v4(),
            listing_id,
            rater_id,
            rated_id,
            3,
            Some("Otra opinión"),
        )
        .await;

    // Assert — should be AlreadyRated (maps to HTTP 409 Conflict)
    assert!(second.is_err(), "Expected Err for duplicate, got {:?}", second);
    match second {
        Err(RatingError::AlreadyRated) => { /* expected — maps to HTTP 409 */ }
        Err(other) => panic!("Expected AlreadyRated, got {:?}", other),
        Ok(_) => panic!("Expected Err, got Ok"),
    }

    // Verify only one rating exists
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ratings WHERE listing_id = $1 AND rater_id = $2")
            .bind(listing_id)
            .bind(rater_id)
            .fetch_one(&pool)
            .await
            .expect("DB query failed");
    assert_eq!(count.0, 1, "Only one rating should exist despite two insert attempts");
}
