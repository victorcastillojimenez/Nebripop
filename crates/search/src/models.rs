use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Geo-coordinates for MeiliSearch `_geo` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geo {
    pub lat: f64,
    pub lng: f64,
}

/// A search result returned from the search engine.
/// This is intentionally NOT a domain entity — it is a read-only projection
/// optimised for display in search/list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: Uuid,
    pub title: String,
    pub price: f64,
    pub currency: String,
    pub category: String,
    pub condition: String,
    pub city: String,
    pub image_url: Option<String>,
    /// Distance in kilometres (None if geo-radius was not queried).
    pub distance_km: Option<f64>,
    pub created_at: i64,
}

/// Filters that can be applied to a search query.
/// All fields are optional; only provided filters are applied.
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    /// Full-text search query string.
    pub query: Option<String>,

    /// Category filter.
    pub category: Option<String>,

    /// Condition filter (one or more values).
    pub condition: Option<Vec<String>>,

    /// Minimum price (inclusive).
    pub min_price: Option<f64>,

    /// Maximum price (inclusive).
    pub max_price: Option<f64>,

    /// Latitude for geo-radius filter.
    pub latitude: Option<f64>,

    /// Longitude for geo-radius filter.
    pub longitude: Option<f64>,

    /// Radius in km for geo filter (default: 50).
    pub radius_km: Option<f64>,

    /// Sort order: "price_asc", "price_desc", "date_desc".
    pub sort: Option<String>,

    /// Page number (0-indexed).
    pub page: i64,

    /// Results per page.
    pub per_page: i64,
}

impl SearchFilters {
    /// Create a new SearchFilters with default pagination.
    pub fn new() -> Self {
        Self {
            query: None,
            category: None,
            condition: None,
            min_price: None,
            max_price: None,
            latitude: None,
            longitude: None,
            radius_km: None,
            sort: None,
            page: 0,
            per_page: 20,
        }
    }

    /// Calculate offset from page and per_page.
    pub fn offset(&self) -> i64 {
        self.page * self.per_page
    }

    /// Get the per_page value clamped to max 100.
    pub fn limit(&self) -> i64 {
        self.per_page.min(100).max(1)
    }
}

impl From<crate::dtos::SearchQueryDto> for SearchFilters {
    fn from(dto: crate::dtos::SearchQueryDto) -> Self {
        Self {
            query: dto.query,
            category: dto.category,
            condition: dto.condition,
            min_price: dto.min_price,
            max_price: dto.max_price,
            latitude: dto.latitude,
            longitude: dto.longitude,
            radius_km: dto.radius_km,
            sort: dto.sort,
            page: dto.page,
            per_page: dto.per_page,
        }
    }
}

/// Document structure sent to MeiliSearch for indexing.
/// This is separate from the domain `Listing` to avoid coupling
/// the search infrastructure to the domain model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingDoc {
    /// The listing ID as a string (MeiliSearch requires string IDs).
    pub id: String,

    pub title: String,
    pub description: String,
    pub price: f64,
    pub currency: String,
    pub category: String,
    pub condition: String,
    pub status: String,
    pub city: String,

    /// Geo-coordinates for `_geoRadius` filter.
    pub _geo: Option<Geo>,

    /// Timestamp as Unix epoch seconds (for sorting).
    pub created_at: i64,

    /// URL of the first image (if any).
    pub image_url: Option<String>,
}
