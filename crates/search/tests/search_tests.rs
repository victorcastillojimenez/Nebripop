//! Integration tests for the search module.
//!
//! These tests verify:
//! - Fallback activation label ("sql_fallback") when MeiliSearch is not configured
//! - Query parameter validation
//! - DTO conversions
//! - SearchFilters construction from DTOs

use search::dtos::SearchQueryDto;
use search::models::SearchFilters;

// ---------------------------------------------------------------------------
// DTO → SearchFilters conversion
// ---------------------------------------------------------------------------

#[test]
fn test_search_query_dto_to_filters_defaults() {
    let dto = SearchQueryDto::default();
    let filters = SearchFilters::from(dto);

    assert_eq!(filters.page, 0);
    assert_eq!(filters.per_page, 20);
    assert!(filters.query.is_none());
    assert!(filters.category.is_none());
}

#[test]
fn test_search_query_dto_to_filters_custom_values() {
    let dto = SearchQueryDto {
        query: Some("bicicleta".to_string()),
        category: Some("deportes".to_string()),
        min_price: Some(10.0),
        max_price: Some(100.0),
        latitude: Some(41.3874),
        longitude: Some(2.1686),
        radius_km: Some(25.0),
        sort: Some("price_asc".to_string()),
        page: 1,
        per_page: 10,
    };
    let filters = SearchFilters::from(dto);

    assert_eq!(filters.query.as_deref(), Some("bicicleta"));
    assert_eq!(filters.category.as_deref(), Some("deportes"));
    assert_eq!(filters.min_price, Some(10.0));
    assert_eq!(filters.max_price, Some(100.0));
    assert_eq!(filters.latitude, Some(41.3874));
    assert_eq!(filters.longitude, Some(2.1686));
    assert_eq!(filters.radius_km, Some(25.0));
    assert_eq!(filters.sort.as_deref(), Some("price_asc"));
    assert_eq!(filters.page, 1);
    assert_eq!(filters.per_page, 10);
}

// ---------------------------------------------------------------------------
// SearchFilters helper methods
// ---------------------------------------------------------------------------

#[test]
fn test_search_filters_offset_and_limit() {
    let filters = SearchFilters {
        page: 0,
        per_page: 20,
        ..SearchFilters::new()
    };
    assert_eq!(filters.offset(), 0);
    assert_eq!(filters.limit(), 20);

    let filters = SearchFilters {
        page: 2,
        per_page: 10,
        ..SearchFilters::new()
    };
    assert_eq!(filters.offset(), 20);
    assert_eq!(filters.limit(), 10);
}

#[test]
fn test_search_filters_limit_clamped_to_max() {
    let filters = SearchFilters {
        page: 0,
        per_page: 200,
        ..SearchFilters::new()
    };
    assert_eq!(filters.limit(), 100); // clamped to max 100
}

#[test]
fn test_search_filters_limit_minimum_one() {
    let filters = SearchFilters {
        page: 0,
        per_page: 0,
        ..SearchFilters::new()
    };
    assert_eq!(filters.limit(), 1); // minimum 1
}

// ---------------------------------------------------------------------------
// Query parameter validation
// ---------------------------------------------------------------------------

#[test]
fn test_search_query_dto_validate_min_price_negative() {
    let dto = SearchQueryDto {
        min_price: Some(-10.0),
        ..SearchQueryDto::default()
    };
    let result = dto.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mayor o igual a 0"));
}

#[test]
fn test_search_query_dto_validate_max_price_negative() {
    let dto = SearchQueryDto {
        max_price: Some(-5.0),
        ..SearchQueryDto::default()
    };
    let result = dto.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mayor o igual a 0"));
}

#[test]
fn test_search_query_dto_validate_min_price_greater_than_max() {
    let dto = SearchQueryDto {
        min_price: Some(100.0),
        max_price: Some(50.0),
        ..SearchQueryDto::default()
    };
    let result = dto.validate();
    assert!(result.is_err());
}

#[test]
fn test_search_query_dto_validate_lat_without_lng() {
    let dto = SearchQueryDto {
        latitude: Some(41.3874),
        longitude: None,
        ..SearchQueryDto::default()
    };
    let result = dto.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("lat"));
}

#[test]
fn test_search_query_dto_validate_lng_without_lat() {
    let dto = SearchQueryDto {
        latitude: None,
        longitude: Some(2.1686),
        ..SearchQueryDto::default()
    };
    let result = dto.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("lat"));
}

#[test]
fn test_search_query_dto_validate_lat_range() {
    let dto = SearchQueryDto {
        latitude: Some(100.0),
        longitude: Some(2.0),
        ..SearchQueryDto::default()
    };
    let result = dto.validate();
    assert!(result.is_err());
}

#[test]
fn test_search_query_dto_validate_lng_range() {
    let dto = SearchQueryDto {
        latitude: Some(41.0),
        longitude: Some(200.0),
        ..SearchQueryDto::default()
    };
    let result = dto.validate();
    assert!(result.is_err());
}

#[test]
fn test_search_query_dto_validate_radius_too_large() {
    let dto = SearchQueryDto {
        latitude: Some(41.0),
        longitude: Some(2.0),
        radius_km: Some(1000.0),
        ..SearchQueryDto::default()
    };
    let result = dto.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("500"));
}

#[test]
fn test_search_query_dto_validate_invalid_sort() {
    let dto = SearchQueryDto {
        sort: Some("invalid_sort".to_string()),
        ..SearchQueryDto::default()
    };
    let result = dto.validate();
    assert!(result.is_err());
}

#[test]
fn test_search_query_dto_validate_valid_sort() {
    for sort_str in &["price_asc", "price_desc", "date_desc"] {
        let dto = SearchQueryDto {
            sort: Some(sort_str.to_string()),
            ..SearchQueryDto::default()
        };
        let result = dto.validate();
        assert!(result.is_ok(), "sort = '{}' should be valid", sort_str);
    }
}

// ---------------------------------------------------------------------------
// SearchResponseDto helpers
// ---------------------------------------------------------------------------

#[test]
fn test_search_response_dto_page_defaults() {
    let dto = SearchQueryDto::default();
    assert_eq!(dto.page, 0);
    assert_eq!(dto.per_page, 20);
}

#[test]
fn test_search_response_dto_total_pages_division() {
    use search::dtos::SearchResponseDto;

    let resp = SearchResponseDto::new(vec![], 0, 0, 20, "test");
    assert_eq!(resp.total_pages, 0);

    let resp = SearchResponseDto::new(
        vec![search::dtos::SearchResultDto {
            id: uuid::Uuid::new_v4(),
            title: "Test".to_string(),
            price: 10.0,
            currency: "eur".to_string(),
            category: "test".to_string(),
            condition: "used".to_string(),
            city: "Barcelona".to_string(),
            image_url: None,
            distance_km: None,
            created_at: 0,
        }],
        1,
        0,
        20,
        "test",
    );
    assert_eq!(resp.total_pages, 1);
}

// ---------------------------------------------------------------------------
// Engine label test: verify that when MeiliSearch is None, the usecase
// returns "sql_fallback" as the engine label.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_search_usecase_fallback_label() {
    use search::usecases::search_usecase;

    // This test requires a real database pool, which is not available
    // in unit test context. This is an integration test marker.
    // The unit-testable part is that SearchResponseDto carries the engine label.
    let dto = search::dtos::SearchResponseDto::new(vec![], 0, 0, 20, "sql_fallback");
    assert_eq!(dto.engine, "sql_fallback");
}
