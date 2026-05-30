//! Integration tests for the listings module.
//!
//! These tests use `#[sqlx::test]` to get an ephemeral PostgreSQL database.
//! They test the complete CRUD flow for listings through the usecase layer:
//! - Create (authenticated user success)
//! - Update (owner → 200, not owner → NotOwner)
//! - Delete soft (via repository directly to avoid ImageStorage dependency)
//! - List (paginated results via repository)
//! - Detail (by ID, nonexistent → NotFound)
//!
//! Pattern: given_<state>_when_<action>_then_<result>

use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use listings::adapters::listing_repository::ListingRepositoryImpl;
use listings::dtos::{CreateListingDto, UpdateListingDto};
use listings::errors::ListingError;
use listings::models::PhysicalCondition;
use listings::ports::ListingRepository;
use listings::usecases::{
    create_listing_usecase,
    get_listing_usecase,
    update_listing_usecase,
};

// ── Helpers ──────────────────────────────────────────────────────────────

fn make_repo(pool: PgPool) -> ListingRepositoryImpl {
    ListingRepositoryImpl::new(pool)
}

/// Create a user in the database and return the user's UUID.
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    let password_hash =
        "$argon2id$v=19$m=19456,t=2,p=1$test_salt$test_hash_value_abcdefghijklmnopqrstuvwxyz0123456789abcdefghij";

    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash, role, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'user', NOW(), NOW())",
    )
    .bind(user_id)
    .bind(email)
    .bind("Test User")
    .bind(password_hash)
    .execute(pool)
    .await
    .expect("Seeding user should succeed");

    user_id
}

/// Seed a listing directly in the database for test setup.
async fn seed_listing(
    pool: &PgPool,
    owner_id: Uuid,
    title: &str,
    description: &str,
    price: Decimal,
    status: &str,
    category: &str,
    lat: f64,
    lng: f64,
) -> Uuid {
    let listing_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO listings (id, seller_id, title, description, price, currency, status, category, condition, location_lat, location_lon, city, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'eur', $6, $7, 'used', $8, $9, 'TestCity', $10, $11)",
    )
    .bind(listing_id)
    .bind(owner_id)
    .bind(title)
    .bind(description)
    .bind(price)
    .bind(status)
    .bind(category)
    .bind(lat)
    .bind(lng)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("Seeding listing should succeed");

    listing_id
}

/// Seed a listing with a specific condition for test setup.
async fn seed_listing_with_condition(
    pool: &PgPool,
    owner_id: Uuid,
    title: &str,
    description: &str,
    price: Decimal,
    status: &str,
    category: &str,
    condition: &str,
    lat: f64,
    lng: f64,
) -> Uuid {
    let listing_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO listings (id, seller_id, title, description, price, currency, status, category, condition, location_lat, location_lon, city, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'eur', $6, $7, $8, $9, $10, 'TestCity', $11, $12)",
    )
    .bind(listing_id)
    .bind(owner_id)
    .bind(title)
    .bind(description)
    .bind(price)
    .bind(status)
    .bind(category)
    .bind(condition)
    .bind(lat)
    .bind(lng)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("Seeding listing with condition should succeed");

    listing_id
}

// ── Tests ────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn given_authenticated_user_when_create_listing_then_persisted(pool: PgPool) {
    let repo = make_repo(pool.clone());
    let owner_id = seed_user(&pool, "create_test@nebripop.test").await;

    let dto = CreateListingDto {
        title: "Bicicleta de montaña".to_string(),
        description: "Bicicleta casi nueva, solo usada 3 veces".to_string(),
        price: Decimal::new(15000, 2), // 150.00
        category: "deportes".to_string(),
        condition: PhysicalCondition::Used,
        location_lat: 19.4326,
        location_lon: -99.1332,
        city: "Ciudad de México".to_string(),
    };

    let result = create_listing_usecase::create_listing_usecase(
        &repo,
        None::<&dyn search::ports::SearchEngine>,
        owner_id,
        dto,
    )
    .await;
    assert!(
        result.is_ok(),
        "Authenticated user should be able to create listing"
    );

    let listing = result.unwrap();
    assert_eq!(listing.title, "Bicicleta de montaña");
    assert_eq!(listing.price, Decimal::new(15000, 2));
    assert_eq!(listing.category, "deportes");

    // Verify in database
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM listings WHERE seller_id = $1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("Query should succeed");
    assert_eq!(count.0, 1, "Exactly one listing should be in the database");
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_valid_dto_when_create_listing_then_usecase_succeeds(pool: PgPool) {
    let repo = make_repo(pool.clone());
    let owner_id = seed_user(&pool, "dto_test@nebripop.test").await;

    let dto = CreateListingDto {
        title: "Mesa de madera".to_string(),
        description: "Mesa robusta de pino".to_string(),
        price: Decimal::new(8500, 2), // 85.00
        category: "muebles".to_string(),
        condition: PhysicalCondition::Used,
        location_lat: 40.4168,
        location_lon: -3.7038,
        city: "Madrid".to_string(),
    };

    let result = create_listing_usecase::create_listing_usecase(
        &repo,
        None::<&dyn search::ports::SearchEngine>,
        owner_id,
        dto,
    )
    .await;
    assert!(result.is_ok(), "Valid DTO should result in success");

    let listing = result.unwrap();
    assert_eq!(listing.title, "Mesa de madera");
    assert_eq!(listing.category, "muebles");
    assert_eq!(listing.price, Decimal::new(8500, 2));
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_owner_when_update_listing_then_updated(pool: PgPool) {
    let repo = make_repo(pool.clone());
    let owner_id = seed_user(&pool, "owner_update@nebripop.test").await;

    let listing_id = seed_listing(
        &pool,
        owner_id,
        "Silla vieja",
        "Silla algo desgastada",
        Decimal::new(5000, 2),
        "active",
        "muebles",
        19.4,
        -99.1,
    )
    .await;

    let update_dto = UpdateListingDto {
        title: Some("Silla restaurada".to_string()),
        description: Some("Silla completamente restaurada y barnizada".to_string()),
        price: Some(Decimal::new(12000, 2)),
        category: Some("antigüedades".to_string()),
        condition: None,
        location_lat: None,
        location_lon: None,
        city: None,
        status: None,
    };

    let result = update_listing_usecase::update_listing_usecase(
        &repo,
        None::<&dyn search::ports::SearchEngine>,
        listing_id,
        owner_id,
        update_dto,
    )
    .await;
    assert!(result.is_ok(), "Owner should be able to update listing");

    let updated = result.unwrap();
    assert_eq!(updated.title, "Silla restaurada");
    assert_eq!(
        updated.description,
        "Silla completamente restaurada y barnizada"
    );
    assert_eq!(updated.price, Decimal::new(12000, 2));
    assert_eq!(updated.category, "antigüedades");

    // Verify in database
    let db_title: (String,) = sqlx::query_as("SELECT title FROM listings WHERE id = $1")
        .bind(listing_id)
        .fetch_one(&pool)
        .await
        .expect("Listing should exist in DB");
    assert_eq!(db_title.0, "Silla restaurada");
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_non_owner_when_update_listing_then_not_owner(pool: PgPool) {
    let repo = make_repo(pool.clone());

    // Owner creates the listing
    let owner_id = seed_user(&pool, "owner@nebripop.test").await;
    let listing_id = seed_listing(
        &pool,
        owner_id,
        "Artículo del dueño",
        "Solo el dueño puede modificar",
        Decimal::new(10000, 2),
        "active",
        "general",
        19.4,
        -99.1,
    )
    .await;

    // Another user tries to update
    let intruder_id = seed_user(&pool, "intruder@nebripop.test").await;

    let update_dto = UpdateListingDto {
        title: Some("Artículo robado".to_string()),
        description: None,
        price: None,
        category: None,
        condition: None,
        location_lat: None,
        location_lon: None,
        city: None,
        status: None,
    };

    let result = update_listing_usecase::update_listing_usecase(
        &repo,
        None::<&dyn search::ports::SearchEngine>,
        listing_id,
        intruder_id,
        update_dto,
    )
    .await;

    match result {
        Err(ListingError::NotOwner(_)) => {} // Expected — non-owner cannot update
        Err(e) => panic!("Expected NotOwner, got {:?}", e),
        Ok(_) => panic!("Non-owner should not be able to update listing"),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_owner_when_soft_delete_then_status_deleted(pool: PgPool) {
    let repo = make_repo(pool.clone());
    let owner_id = seed_user(&pool, "delete_owner@nebripop.test").await;

    let listing_id = seed_listing(
        &pool,
        owner_id,
        "Para eliminar",
        "Este artículo será eliminado",
        Decimal::new(3000, 2),
        "active",
        "general",
        19.4,
        -99.1,
    )
    .await;

    // Test soft delete directly via repository (avoids ImageStorage dependency)
    let result = repo.soft_delete(listing_id).await;
    assert!(result.is_ok(), "Owner should be able to delete listing");

    // Verify soft delete in database
    let status: (String,) = sqlx::query_as("SELECT status FROM listings WHERE id = $1")
        .bind(listing_id)
        .fetch_one(&pool)
        .await
        .expect("Listing should still exist in DB");
    assert_eq!(status.0, "deleted", "Listing should be soft-deleted");
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_listings_when_find_all_paginated_then_returns_active_only(pool: PgPool) {
    let repo = make_repo(pool.clone());
    let owner_id = seed_user(&pool, "list_test@nebripop.test").await;

    // Seed multiple listings
    seed_listing(
        &pool,
        owner_id,
        "Artículo A",
        "Descripción A",
        Decimal::new(1000, 2),
        "active",
        "general",
        19.4,
        -99.1,
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Artículo B",
        "Descripción B",
        Decimal::new(2000, 2),
        "active",
        "general",
        19.4,
        -99.1,
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Artículo C",
        "Descripción C",
        Decimal::new(3000, 2),
        "deleted",
        "general",
        19.4,
        -99.1,
    )
    .await; // Should NOT appear in active listings

    let result = repo.find_all_paginated(0, 20, None, None, None, None).await;
    assert!(result.is_ok(), "List should succeed");

    let (listings, total) = result.unwrap();
    assert_eq!(total, 2, "Only 2 active listings should be returned");
    assert_eq!(listings.len(), 2, "Response should contain 2 listings");

    let titles: Vec<&str> = listings.iter().map(|l| l.title.as_str()).collect();
    assert!(titles.contains(&"Artículo A"));
    assert!(titles.contains(&"Artículo B"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_listing_id_when_get_detail_then_returns_listing(pool: PgPool) {
    let repo = make_repo(pool.clone());
    let owner_id = seed_user(&pool, "detail_test@nebripop.test").await;

    let listing_id = seed_listing(
        &pool,
        owner_id,
        "Detalle artículo",
        "Descripción detallada del artículo",
        Decimal::new(7500, 2),
        "active",
        "electrónica",
        19.4,
        -99.1,
    )
    .await;

    let result = get_listing_usecase::get_listing_usecase(&repo, listing_id).await;
    assert!(result.is_ok(), "Existing listing should be found");

    let listing = result.unwrap();
    assert_eq!(listing.title, "Detalle artículo");
    assert_eq!(listing.description, "Descripción detallada del artículo");
    assert_eq!(listing.price, Decimal::new(7500, 2));
    assert_eq!(listing.category, "electrónica");
    assert_eq!(listing.location_lat, 19.4);
    assert_eq!(listing.location_lon, -99.1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_nonexistent_id_when_get_listing_then_returns_not_found(pool: PgPool) {
    let repo = make_repo(pool);
    let fake_id = Uuid::new_v4();

    let result = get_listing_usecase::get_listing_usecase(&repo, fake_id).await;

    match result {
        Err(ListingError::NotFound(_)) => {} // Expected
        Err(e) => panic!("Expected NotFound, got {:?}", e),
        Ok(_) => panic!("Nonexistent listing should return NotFound"),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_category_filter_when_find_all_paginated_then_filters_correctly(pool: PgPool) {
    let repo = make_repo(pool.clone());
    let owner_id = seed_user(&pool, "cat_filter@nebripop.test").await;

    seed_listing(
        &pool,
        owner_id,
        "Zapatillas",
        "Zapatillas de running",
        Decimal::new(8000, 2),
        "active",
        "deportes",
        19.4,
        -99.1,
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Libro",
        "Novela de ciencia ficción",
        Decimal::new(1500, 2),
        "active",
        "libros",
        19.4,
        -99.1,
    )
    .await;

    let result = repo.find_all_paginated(0, 20, Some("deportes"), None, None, None).await;
    assert!(result.is_ok(), "Category filter should succeed");

    let (listings, total) = result.unwrap();
    assert_eq!(total, 1, "Only 1 listing in 'deportes' category");
    assert_eq!(listings[0].category, "deportes");
    assert_eq!(listings[0].title, "Zapatillas");
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_condition_filter_when_find_all_paginated_then_filters_correctly(pool: PgPool) {
    let repo = make_repo(pool.clone());
    let owner_id = seed_user(&pool, "cond_filter@nebripop.test").await;

    // Seed a 'new' listing
    seed_listing_with_condition(
        &pool,
        owner_id,
        "Teléfono nuevo",
        "Teléfono sin estrenar",
        Decimal::new(50000, 2),
        "active",
        "tecnologia",
        "new",
        19.4,
        -99.1,
    )
    .await;

    // Seed a 'used' listing
    seed_listing_with_condition(
        &pool,
        owner_id,
        "Mochila usada",
        "Mochila de segunda mano",
        Decimal::new(2000, 2),
        "active",
        "hogar",
        "used",
        19.4,
        -99.1,
    )
    .await;

    // Seed a 'like_new' listing
    seed_listing_with_condition(
        &pool,
        owner_id,
        "Libro como nuevo",
        "Libro casi sin usar",
        Decimal::new(1500, 2),
        "active",
        "libros",
        "like_new",
        19.4,
        -99.1,
    )
    .await;

    // Filter by 'new'
    let result = repo        .find_all_paginated(0, 20, None, Some("new"), None, None).await;
    assert!(result.is_ok(), "Condition filter should succeed");

    let (listings, total) = result.unwrap();
    assert_eq!(total, 1, "Only 1 listing with condition 'new'");
    assert_eq!(listings[0].title, "Teléfono nuevo");
    assert_eq!(listings[0].condition.as_str(), "new");

    // Filter by 'used'
    let result = repo.find_all_paginated(0, 20, None, Some("used"), None, None).await;
    assert!(result.is_ok(), "Condition filter should succeed");

    let (listings, total) = result.unwrap();
    assert_eq!(total, 1, "Only 1 listing with condition 'used'");
    assert_eq!(listings[0].title, "Mochila usada");
    assert_eq!(listings[0].condition.as_str(), "used");

    // Filter by 'like_new'
    let result = repo.find_all_paginated(0, 20, None, Some("like_new"), None, None).await;
    assert!(result.is_ok(), "Condition filter should succeed");

    let (listings, total) = result.unwrap();
    assert_eq!(total, 1, "Only 1 listing with condition 'like_new'");
    assert_eq!(listings[0].title, "Libro como nuevo");
    assert_eq!(listings[0].condition.as_str(), "like_new");

    // Combined filter: condition='new' + category='tecnologia'
    let result = repo.find_all_paginated(0, 20, Some("tecnologia"), Some("new"), None, None).await;
    assert!(result.is_ok(), "Combined filter should succeed");

    let (listings, total) = result.unwrap();
    assert_eq!(total, 1, "Only 1 listing matching both filters");
    assert_eq!(listings[0].title, "Teléfono nuevo");

    // Combined filter that should return empty
    let result = repo.find_all_paginated(0, 20, Some("hogar"), Some("new"), None, None).await;
    assert!(result.is_ok(), "Combined filter should succeed");

    let (listings, total) = result.unwrap();
    assert_eq!(total, 0, "No listings match 'hogar' + 'new'");
    assert!(listings.is_empty());
}
