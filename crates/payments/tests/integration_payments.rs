//! Integration tests for the payments module.
//!
//! Tests cover:
//! - Database persistence of payments via PostgresPaymentRepository
//! - Stripe webhook signature verification (invalid → 400)
//! - Self-purchase validation (buyer == seller → 422)
//!
//! Database-dependent tests use `#[sqlx::test]` with ephemeral DB.
//! Signature verification uses the real StripeAdapter with test vectors.

use sqlx::PgPool;
use uuid::Uuid;

use payments::adapters::payment_repository::PostgresPaymentRepository;
use payments::adapters::stripe_adapter::StripeAdapter;
use payments::errors::PaymentError;
use payments::models::{Payment, PaymentStatus};
use payments::ports::{PaymentRepository, StripePort};

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

/// Creates a minimal active listing row.
async fn fixture_listing(pool: &PgPool, seller_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO listings (id, seller_id, title, description, price, category, condition, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'active')",
    )
    .bind(id)
    .bind(seller_id)
    .bind("Producto de prueba")
    .bind("Descripción")
    .bind(rust_decimal::Decimal::new(5000, 2)) // 50.00
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
async fn given_valid_payment_when_insert_then_persisted(pool: PgPool) {
    // Arrange
    let buyer_id = fixture_user(&pool, "buyer_pay").await;
    let seller_id = fixture_user(&pool, "seller_pay").await;
    let listing_id = fixture_listing(&pool, seller_id).await;

    let repo = PostgresPaymentRepository::new(pool.clone());

    let payment = Payment {
        id: Uuid::new_v4(),
        listing_id,
        buyer_id,
        seller_id,
        stripe_payment_intent_id: "pi_test_integration_123".to_string(),
        amount_cents: 5000,
        currency: "eur".to_string(),
        status: PaymentStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Act
    let result = repo.insert(&payment).await;

    // Assert
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    let saved = result.unwrap();
    assert_eq!(saved.stripe_payment_intent_id, "pi_test_integration_123");
    assert_eq!(saved.status, PaymentStatus::Pending);
    assert_eq!(saved.amount_cents, 5000);

    // Verify via direct query
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM payments WHERE stripe_payment_intent_id = $1")
            .bind("pi_test_integration_123")
            .fetch_one(&pool)
            .await
            .expect("DB query failed");
    assert_eq!(count.0, 1, "Payment should be persisted in DB");
}

#[sqlx::test(migrations = "../../migrations/")]
async fn given_payment_when_update_status_then_status_changed(pool: PgPool) {
    // Arrange
    let buyer_id = fixture_user(&pool, "buyer_upd").await;
    let seller_id = fixture_user(&pool, "seller_upd").await;
    let listing_id = fixture_listing(&pool, seller_id).await;

    let repo = PostgresPaymentRepository::new(pool.clone());

    let payment = Payment {
        id: Uuid::new_v4(),
        listing_id,
        buyer_id,
        seller_id,
        stripe_payment_intent_id: "pi_test_update_status".to_string(),
        amount_cents: 3000,
        currency: "eur".to_string(),
        status: PaymentStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    repo.insert(&payment)
        .await
        .expect("Setup: failed to insert payment");

    // Act
    let update_result = repo
        .update_status("pi_test_update_status", PaymentStatus::Succeeded)
        .await;

    // Assert
    assert!(update_result.is_ok(), "Expected Ok, got {:?}", update_result);

    // Verify via direct query
    let status: (String,) = sqlx::query_as(
        "SELECT status FROM payments WHERE stripe_payment_intent_id = $1",
    )
    .bind("pi_test_update_status")
    .fetch_one(&pool)
    .await
    .expect("DB query failed");
    assert_eq!(status.0, "succeeded", "Status should be updated to 'succeeded'");
}

// ---------------------------------------------------------------------------
// Webhook signature verification (no DB needed)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_invalid_signature_when_verify_webhook_then_returns_400() {
    // We create a StripeAdapter with a known webhook secret and test that
    // a tampered signature is correctly rejected (→ InvalidSignature → 400).
    let adapter = StripeAdapter::new(
        "sk_test_dummy".to_string(),
        "whsec_test_secret".to_string(),
    );

    let payload = br#"{"type":"payment_intent.succeeded","data":{"object":{"id":"pi_test"}}}"#;
    let bad_signature = "t=1234567890,v1=tampered_signature_value";

    let result = adapter.verify_webhook_signature(payload, bad_signature);

    assert!(
        result.is_err(),
        "Expected Err for invalid signature, got {:?}",
        result
    );
    match result {
        Err(PaymentError::InvalidSignature) => { /* expected — maps to HTTP 400 */ }
        Err(other) => panic!("Expected InvalidSignature, got {:?}", other),
        Ok(_) => panic!("Expected Err, got Ok"),
    }
}

#[tokio::test]
async fn given_missing_signature_header_when_verify_webhook_then_returns_400() {
    let adapter = StripeAdapter::new(
        "sk_test_dummy".to_string(),
        "whsec_test_secret".to_string(),
    );

    let payload = br#"{"type":"payment_intent.succeeded","data":{"object":{"id":"pi_test"}}}"#;
    // Missing both t= and v1= parts
    let malformed_header = "not_a_valid_signature";

    let result = adapter.verify_webhook_signature(payload, malformed_header);

    assert!(result.is_err(), "Expected Err for missing signature parts");
    match result {
        Err(PaymentError::InvalidSignature) => { /* expected — maps to HTTP 400 */ }
        Err(other) => panic!("Expected InvalidSignature, got {:?}", other),
        Ok(_) => panic!("Expected Err, got Ok"),
    }
}

// ---------------------------------------------------------------------------
// Self-purchase validation (buyer == seller → 422)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_buyer_equals_seller_when_create_intent_then_returns_422() {
    // This tests the business rule at the usecase level.
    // The usecase returns PaymentError::SelfPurchase which maps to 422.
    let buyer_id = Uuid::new_v4();
    let seller_id = buyer_id; // Same user → self-purchase

    // Use the real listing adapter with a mock-like approach:
    // we create a mock ListingService for the test
    use payments::usecases::create_intent_usecase::{ListingInfo, ListingService};
    use async_trait::async_trait;

    struct MockListingService {
        listing: Option<ListingInfo>,
    }

    #[async_trait]
    impl ListingService for MockListingService {
        async fn find_active_by_id(
            &self,
            _id: Uuid,
        ) -> Result<Option<ListingInfo>, PaymentError> {
            Ok(self.listing.clone())
        }
    }

    struct MockStripePort;

    #[async_trait]
    impl payments::ports::StripePort for MockStripePort {
        async fn create_payment_intent(
            &self,
            _amount_cents: i64,
            _currency: &str,
            _listing_id: Uuid,
            _buyer_id: Uuid,
            _idempotency_key: Uuid,
        ) -> Result<(String, String), PaymentError> {
            Ok(("pi_mock".to_string(), "cs_mock".to_string()))
        }

        fn verify_webhook_signature(
            &self,
            _payload: &[u8],
            _signature_header: &str,
        ) -> Result<String, PaymentError> {
            unreachable!()
        }
    }

    struct MockPaymentRepo;

    #[async_trait]
    impl payments::ports::PaymentRepository for MockPaymentRepo {
        async fn insert(&self, _payment: &Payment) -> Result<Payment, PaymentError> {
            unreachable!()
        }

        async fn update_status(
            &self,
            _stripe_intent_id: &str,
            _new_status: PaymentStatus,
        ) -> Result<(), PaymentError> {
            unreachable!()
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<Payment>, PaymentError> {
            unreachable!()
        }

        async fn find_by_stripe_intent_id(
            &self,
            _stripe_intent_id: &str,
        ) -> Result<Option<Payment>, PaymentError> {
            unreachable!()
        }
    }

    let listing_service = MockListingService {
        listing: Some(ListingInfo {
            seller_id,
            amount_cents: 5000,
        }),
    };

    let dto = payments::dtos::CreateIntentDto {
        listing_id: Uuid::new_v4(),
        currency: "eur".to_string(),
    };

    // Act
    let result = payments::usecases::create_intent_usecase::create_intent_usecase(
        dto,
        buyer_id,
        &listing_service,
        &MockPaymentRepo,
        &MockStripePort,
    )
    .await;

    // Assert — SelfPurchase maps to AppError::UnprocessableEntity which is 422
    assert!(result.is_err(), "Expected Err for self-purchase");
    match result {
        Err(PaymentError::SelfPurchase) => { /* expected — maps to HTTP 422 */ }
        Err(other) => panic!("Expected SelfPurchase, got {:?}", other),
        Ok(_) => panic!("Expected Err, got Ok"),
    }
}
