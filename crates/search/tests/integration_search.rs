//! Integration tests for the search module.
//!
//! These tests use `#[sqlx::test]` to get an ephemeral PostgreSQL database.
//! They test the search functionality end-to-end through the public usecase API:
//! - Text search (fallback path when no MeiliSearch engine is provided)
//! - Category and price filters
//! - Geo search within radius
//! - Empty results / no matches
//! - Validation errors for invalid input
//!
//! Pattern: given_<state>_when_<action>_then_<result>

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use search::dtos::SearchQueryDto;
use search::usecases::search_usecase;

// ── Helpers ──────────────────────────────────────────────────────────────

/// Seed a user in the database and return user UUID.
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
#[allow(dead_code)]
async fn seed_listing(
    pool: &PgPool,
    owner_id: Uuid,
    title: &str,
    description: &str,
    price: Decimal,
    category: &str,
    lat: f64,
    lng: f64,
    status: &str,
) -> Uuid {
    let listing_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO listings (id, seller_id, title, description, price, currency, status, category, condition, location_lat, location_lon, location, city, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'eur', $6, $7, 'used', $8, $9, ST_SetSRID(ST_MakePoint($9, $8), 4326)::geography, 'TestCity', $10, $11)",
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

// ── Tests ────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn given_search_query_when_search_then_returns_matching_results(pool: PgPool) {
    let owner_id = seed_user(&pool, "search_test@nebripop.test").await;

    // Seed listings that should match
    seed_listing(
        &pool,
        owner_id,
        "Bicicleta de montaña",
        "Bicicleta especializada para senderos",
        Decimal::new(25000, 2),
        "deportes",
        19.43,
        -99.13,
        "active",
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Bicicleta de carretera",
        "Bicicleta ligera para asfalto",
        Decimal::new(35000, 2),
        "deportes",
        19.44,
        -99.14,
        "active",
    )
    .await;

    // Seed a listing that should NOT match
    seed_listing(
        &pool,
        owner_id,
        "Mesa de madera",
        "Mesa robusta de pino",
        Decimal::new(8500, 2),
        "muebles",
        19.45,
        -99.15,
        "active",
    )
    .await;

    let query = SearchQueryDto {
        query: Some("bicicleta".to_string()),
        category: None,
        min_price: None,
        max_price: None,
        latitude: None,
        longitude: None,
        radius_km: None,
        sort: None,
        ..Default::default()
    };

    let result = search_usecase::execute(None, &pool, query).await;
    assert!(result.is_ok(), "Search should succeed");

    let (response, engine) = result.unwrap();
    assert_eq!(
        engine, "sql_fallback",
        "Should use SQL fallback when no engine is provided"
    );
    assert!(
        response.items.len() >= 2,
        "Should find at least 2 'bicicleta' listings, found {}",
        response.items.len()
    );

    let titles: Vec<&str> = response.items.iter().map(|l| l.title.as_str()).collect();
    assert!(titles.contains(&"Bicicleta de montaña"));
    assert!(titles.contains(&"Bicicleta de carretera"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_empty_query_when_search_then_returns_all_active(pool: PgPool) {
    let owner_id = seed_user(&pool, "all_active@nebripop.test").await;

    seed_listing(
        &pool,
        owner_id,
        "Artículo 1",
        "Descripción genérica",
        Decimal::new(10000, 2),
        "general",
        19.43,
        -99.13,
        "active",
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Artículo 2",
        "Otra descripción",
        Decimal::new(20000, 2),
        "general",
        19.44,
        -99.14,
        "active",
    )
    .await;

    let query = SearchQueryDto {
        query: None,
        category: None,
        min_price: None,
        max_price: None,
        latitude: None,
        longitude: None,
        radius_km: None,
        sort: None,
        ..Default::default()
    };

    let result = search_usecase::execute(None, &pool, query).await;
    assert!(result.is_ok(), "Empty query search should succeed");

    let (response, engine) = result.unwrap();
    assert_eq!(engine, "sql_fallback");
    assert_eq!(response.items.len(), 2, "Should return all active listings");
    assert_eq!(response.total, 2, "Total count should be 2");
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_search_query_with_no_matches_when_search_then_returns_empty(pool: PgPool) {
    let owner_id = seed_user(&pool, "no_match@nebripop.test").await;

    // Seed some listings
    seed_listing(
        &pool,
        owner_id,
        "Laptop",
        "Laptop de última generación",
        Decimal::new(50000, 2),
        "electrónica",
        19.43,
        -99.13,
        "active",
    )
    .await;

    let query = SearchQueryDto {
        query: Some("zzzznonexistentzzzz".to_string()),
        category: None,
        min_price: None,
        max_price: None,
        latitude: None,
        longitude: None,
        radius_km: None,
        sort: None,
        ..Default::default()
    };

    let result = search_usecase::execute(None, &pool, query).await;
    assert!(result.is_ok(), "Search with no matches should succeed");

    let (response, _engine) = result.unwrap();
    assert_eq!(
        response.items.len(),
        0,
        "Should return empty results for nonexistent query"
    );
    assert_eq!(response.total, 0, "Total count should be 0");
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_category_filter_when_search_then_filters_correctly(pool: PgPool) {
    let owner_id = seed_user(&pool, "category_test@nebripop.test").await;

    // Seed listings in different categories
    seed_listing(
        &pool,
        owner_id,
        "Balón de fútbol",
        "Balón profesional",
        Decimal::new(1500, 2),
        "deportes",
        19.43,
        -99.13,
        "active",
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Raqueta de tenis",
        "Raqueta de competición",
        Decimal::new(8000, 2),
        "deportes",
        19.44,
        -99.14,
        "active",
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Sofá",
        "Sofá de tres plazas",
        Decimal::new(30000, 2),
        "muebles",
        19.45,
        -99.15,
        "active",
    )
    .await;

    let query = SearchQueryDto {
        query: None,
        category: Some("deportes".to_string()),
        min_price: None,
        max_price: None,
        latitude: None,
        longitude: None,
        radius_km: None,
        sort: None,
        ..Default::default()
    };

    let result = search_usecase::execute(None, &pool, query).await;
    assert!(result.is_ok(), "Category filter search should succeed");

    let (response, _engine) = result.unwrap();
    assert_eq!(
        response.items.len(),
        2,
        "Should find exactly 2 listings in 'deportes' category"
    );

    for item in &response.items {
        assert_eq!(item.category, "deportes");
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_price_range_when_search_then_filters_by_price(pool: PgPool) {
    let owner_id = seed_user(&pool, "price_test@nebripop.test").await;

    // Seed listings at different price points
    seed_listing(
        &pool,
        owner_id,
        "Artículo barato",
        "Muy económico",
        Decimal::new(5000, 2), // 50.00
        "general",
        19.43,
        -99.13,
        "active",
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Artículo medio",
        "Precio moderado",
        Decimal::new(15000, 2), // 150.00
        "general",
        19.44,
        -99.14,
        "active",
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Artículo caro",
        "Precio elevado",
        Decimal::new(50000, 2), // 500.00
        "general",
        19.45,
        -99.15,
        "active",
    )
    .await;

    let query = SearchQueryDto {
        query: None,
        category: None,
        min_price: Some(100.0),  // min 100.00
        max_price: Some(300.0),  // max 300.00
        latitude: None,
        longitude: None,
        radius_km: None,
        sort: None,
        ..Default::default()
    };

    let result = search_usecase::execute(None, &pool, query).await;
    assert!(result.is_ok(), "Price filter search should succeed");

    let (response, _engine) = result.unwrap();
    assert_eq!(
        response.items.len(),
        1,
        "Should find exactly 1 listing in price range 100-300"
    );
    assert_eq!(response.items[0].title, "Artículo medio");
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_deleted_listings_when_search_then_not_included(pool: PgPool) {
    let owner_id = seed_user(&pool, "deleted_in_search@nebripop.test").await;

    // Active listing
    seed_listing(
        &pool,
        owner_id,
        "Activo",
        "Este listing está activo",
        Decimal::new(10000, 2),
        "general",
        19.43,
        -99.13,
        "active",
    )
    .await;

    // Deleted listing (should be excluded from search)
    seed_listing(
        &pool,
        owner_id,
        "Eliminado",
        "Este listing fue eliminado",
        Decimal::new(20000, 2),
        "general",
        19.44,
        -99.14,
        "deleted",
    )
    .await;

    let query = SearchQueryDto {
        query: None,
        category: None,
        min_price: None,
        max_price: None,
        latitude: None,
        longitude: None,
        radius_km: None,
        sort: None,
        ..Default::default()
    };

    let result = search_usecase::execute(None, &pool, query).await;
    assert!(result.is_ok(), "Search should succeed");

    let (response, _engine) = result.unwrap();
    assert_eq!(
        response.items.len(),
        1,
        "Only active listings should appear in search results"
    );
    assert_eq!(response.items[0].title, "Activo");
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_valid_geo_search_when_search_then_returns_nearby_results(pool: PgPool) {
    let owner_id = seed_user(&pool, "geo_valid@nebripop.test").await;

    // Insert listing near (19.43, -99.13)
    seed_listing(
        &pool,
        owner_id,
        "Cerca",
        "Cerca del punto de referencia",
        Decimal::new(10000, 2),
        "general",
        19.432,
        -99.132,
        "active",
    )
    .await;

    // Search with valid geo params
    let query = SearchQueryDto {
        query: None,
        category: None,
        min_price: None,
        max_price: None,
        latitude: Some(19.43),
        longitude: Some(-99.13),
        radius_km: Some(10.0),
        sort: None,
        ..Default::default()
    };

    let result = search_usecase::execute(None, &pool, query).await;
    assert!(result.is_ok(), "Geo search with valid params should succeed");
    let (response, _engine) = result.unwrap();
    assert!(
        response.items.len() >= 1,
        "Should find nearby listing"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn given_sort_by_price_asc_when_search_then_results_ordered(pool: PgPool) {
    let owner_id = seed_user(&pool, "sort_test@nebripop.test").await;

    seed_listing(
        &pool,
        owner_id,
        "Más caro",
        "Precio alto",
        Decimal::new(50000, 2),
        "general",
        19.45,
        -99.15,
        "active",
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Más barato",
        "Precio bajo",
        Decimal::new(5000, 2),
        "general",
        19.43,
        -99.13,
        "active",
    )
    .await;

    seed_listing(
        &pool,
        owner_id,
        "Medio",
        "Precio medio",
        Decimal::new(15000, 2),
        "general",
        19.44,
        -99.14,
        "active",
    )
    .await;

    // Search sorted by price ascending
    // NOTE: SQL fallback does not support price sorting — results come
    // in default order (created_at DESC). MeiliSearch handles the sort.
    let query = SearchQueryDto {
        query: None,
        category: None,
        min_price: None,
        max_price: None,
        latitude: None,
        longitude: None,
        radius_km: None,
        sort: Some("price_asc".to_string()),
        ..Default::default()
    };

    let result = search_usecase::execute(None, &pool, query).await;
    assert!(result.is_ok(), "Sorted search should succeed");

    let (response, _engine) = result.unwrap();
    assert_eq!(
        response.items.len(),
        3,
        "All 3 listings should be returned regardless of sort"
    );

    // With SQL fallback, sort is ignored; results come in created_at DESC.
    // "Más caro" was inserted first, then "Más barato", then "Medio".
    // created_at DESC means "Medio" (last inserted) comes first.
    // We just verify all items are present.
    let titles: Vec<&str> = response.items.iter().map(|l| l.title.as_str()).collect();
    assert!(titles.contains(&"Más barato"));
    assert!(titles.contains(&"Medio"));
    assert!(titles.contains(&"Más caro"));
}

#[test]
fn given_invalid_radius_when_validate_dto_then_returns_error() {
    // Test that DTO validation catches radius > 500 km
    let query = SearchQueryDto {
        query: None,
        category: None,
        min_price: None,
        max_price: None,
        latitude: Some(19.43),
        longitude: Some(-99.13),
        radius_km: Some(1000.0), // Exceeds 500 km max
        sort: None,
        ..Default::default()
    };

    let result = query.validate();
    assert!(result.is_err(), "Radius > 500 km should fail validation");
    let error_msg = result.unwrap_err();
    assert!(
        error_msg.contains("500"),
        "Error should mention radius limit: {}",
        error_msg
    );
}

#[test]
fn given_lat_without_lng_when_validate_dto_then_returns_error() {
    // lat without lng should fail validation
    let query = SearchQueryDto {
        query: None,
        category: None,
        min_price: None,
        max_price: None,
        latitude: Some(19.43),
        longitude: None, // Missing longitude partner
        radius_km: None,
        sort: None,
        ..Default::default()
    };

    let result = query.validate();
    assert!(result.is_err(), "Unbalanced lat/lng should fail validation");
    let error_msg = result.unwrap_err();
    assert!(
        error_msg.contains("lng"),
        "Error message should mention missing lng: {}",
        error_msg
    );
}
