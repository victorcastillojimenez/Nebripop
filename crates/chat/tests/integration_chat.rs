//! Integration tests for the chat module.
//!
//! These tests exercise the real repositories (ConversationRepository,
//! MessageRepository) against an ephemeral PostgreSQL database managed
//! by `#[sqlx::test]`. Fixture data (users, listings) is created before
//! each test to satisfy foreign key constraints.
//!
//! Naming convention: given_<state>_when_<action>_then_<result> (per QA rules)

use sqlx::PgPool;
use uuid::Uuid;

use chat::adapters::conversation_repo::ConversationRepository;
use chat::adapters::message_repo::MessageRepository;
use chat::connections::ActiveConnections;
use chat::dtos::{CreateConversationDto, SendMessageDto};
use chat::errors::ChatError;
use chat::usecases;

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
    .bind("Descripción del producto de prueba")
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
async fn given_valid_participants_when_create_conversation_then_succeeds(pool: PgPool) {
    // Arrange
    let buyer_id = fixture_user(&pool, "buyer_conv").await;
    let seller_id = fixture_user(&pool, "seller_conv").await;
    let listing_id = fixture_listing(&pool, seller_id).await;

    let conv_repo = ConversationRepository::new(pool.clone());
    let msg_repo = MessageRepository::new(pool.clone());

    let dto = CreateConversationDto {
        listing_id,
        initial_message: "Hola, me interesa este producto".to_string(),
    };

    // Act
    let result =
        usecases::create_conversation_usecase::execute(&conv_repo, &msg_repo, buyer_id, dto).await;

    // Assert
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.listing_id, listing_id);
    assert_eq!(
        response.last_message.as_deref(),
        Some("Hola, me interesa este producto")
    );
}

#[sqlx::test(migrations = "../../migrations/")]
async fn given_member_when_send_message_then_message_persisted(pool: PgPool) {
    // Arrange
    let buyer_id = fixture_user(&pool, "buyer_msg").await;
    let seller_id = fixture_user(&pool, "seller_msg").await;
    let listing_id = fixture_listing(&pool, seller_id).await;

    let conv_repo = ConversationRepository::new(pool.clone());
    let msg_repo = MessageRepository::new(pool.clone());
    let connections = ActiveConnections::new();

    // Create a conversation first (via usecase)
    let create_dto = CreateConversationDto {
        listing_id,
        initial_message: "Hola".to_string(),
    };
    let conv = usecases::create_conversation_usecase::execute(
        &conv_repo,
        &msg_repo,
        buyer_id,
        create_dto,
    )
    .await
    .expect("Setup: failed to create conversation");
    let conv_id = conv.id;

    let send_dto = SendMessageDto {
        content: "Sí, está disponible".to_string(),
    };

    // Act
    let result = usecases::send_message_usecase::execute(
        &conv_repo,
        &msg_repo,
        &connections,
        conv_id,
        seller_id,
        send_dto,
    )
    .await;

    // Assert
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    let msg = result.unwrap();
    assert_eq!(msg.content, "Sí, está disponible");
    assert_eq!(msg.sender_id, seller_id);
    assert_eq!(msg.conversation_id, conv_id);

    // Verify in DB directly
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages WHERE conversation_id = $1")
        .bind(conv_id)
        .fetch_one(&pool)
        .await
        .expect("DB query failed");
    assert_eq!(count.0, 2, "Expected 2 messages (initial + new)");
}

#[sqlx::test(migrations = "../../migrations/")]
async fn given_non_member_when_get_messages_then_returns_403(pool: PgPool) {
    // Arrange
    let buyer_id = fixture_user(&pool, "buyer_nm").await;
    let seller_id = fixture_user(&pool, "seller_nm").await;
    let stranger_id = fixture_user(&pool, "stranger_nm").await;
    let listing_id = fixture_listing(&pool, seller_id).await;

    let conv_repo = ConversationRepository::new(pool.clone());
    let msg_repo = MessageRepository::new(pool.clone());

    // Create conversation between buyer and seller
    let create_dto = CreateConversationDto {
        listing_id,
        initial_message: "Hola".to_string(),
    };
    let conv = usecases::create_conversation_usecase::execute(
        &conv_repo,
        &msg_repo,
        buyer_id,
        create_dto,
    )
    .await
    .expect("Setup: failed to create conversation");
    let conv_id = conv.id;

    // Act — stranger tries to access messages
    let result = usecases::get_messages_usecase::execute(
        &conv_repo,
        &msg_repo,
        conv_id,
        stranger_id,
        None,
        50,
    )
    .await;

    // Assert — should be NotMember (maps to HTTP 403)
    assert!(result.is_err(), "Expected Err, got {:?}", result);
    match result {
        Err(ChatError::NotMember(id)) => {
            assert_eq!(id, stranger_id, "NotMember error should contain stranger ID");
        }
        Err(other) => panic!("Expected NotMember, got {:?}", other),
        Ok(_) => panic!("Expected Err, got Ok"),
    }
}
