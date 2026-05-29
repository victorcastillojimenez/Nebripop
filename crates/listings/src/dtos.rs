use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::models::{Listing, ListingImage, ListingStatus, PhysicalCondition};

// ──────────────────────────────────────────────
//  Request DTOs
// ──────────────────────────────────────────────

/// DTO for creating a new listing.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateListingDto {
    /// Title of the listing (3-100 characters).
    #[validate(length(min = 3, max = 100, message = "El título debe tener entre 3 y 100 caracteres"))]
    pub title: String,

    /// Detailed description of the item (max 2000 characters).
    #[validate(length(max = 2000, message = "La descripción no puede exceder los 2000 caracteres"))]
    pub description: String,

    /// Price in EUR (must be > 0 and <= 999999.99).
    pub price: Decimal,

    /// Product category.
    #[validate(length(min = 1, max = 50, message = "La categoría debe tener entre 1 y 50 caracteres"))]
    pub category: String,

    /// Physical condition of the item.
    pub condition: PhysicalCondition,

    /// Latitude of the listing location.
    pub location_lat: f64,

    /// Longitude of the listing location.
    pub location_lon: f64,

    /// City name for display.
    #[validate(length(min = 1, max = 100, message = "La ciudad debe tener entre 1 y 100 caracteres"))]
    pub city: String,
}

/// DTO for updating an existing listing (PATCH semantics).
/// All fields are optional; only provided fields will be updated.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateListingDto {
    pub title: Option<String>,
    pub description: Option<String>,
    pub price: Option<Decimal>,
    pub category: Option<String>,
    pub condition: Option<PhysicalCondition>,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
    pub city: Option<String>,
}

/// Pagination query parameters.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,

    /// Optional category filter
    pub category: Option<String>,

    /// Optional status filter
    pub seller_id: Option<Uuid>,
}

fn default_page() -> i64 {
    0
}

fn default_per_page() -> i64 {
    20
}

// ──────────────────────────────────────────────
//  Response DTOs
// ──────────────────────────────────────────────

/// Full listing response, used for detail views.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingResponseDto {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub title: String,
    pub description: String,
    pub price: Decimal,
    pub currency: String,
    pub category: String,
    pub condition: PhysicalCondition,
    pub status: ListingStatus,
    pub location_lat: f64,
    pub location_lon: f64,
    pub city: String,
    pub images: Vec<ListingImageResponseDto>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Summary version of a listing for list views (no description).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingSummaryDto {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub title: String,
    pub price: Decimal,
    pub currency: String,
    pub category: String,
    pub condition: PhysicalCondition,
    pub status: ListingStatus,
    pub city: String,
    pub first_image_url: Option<String>,
    pub image_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Image response DTO.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingImageResponseDto {
    pub id: Uuid,
    pub image_url: String,
    pub position: i32,
}

/// Paginated response wrapper.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: i64, per_page: i64, total: i64) -> Self {
        Self {
            data,
            page,
            per_page,
            total,
        }
    }
}

// ──────────────────────────────────────────────
//  Conversions
// ──────────────────────────────────────────────

impl From<ListingImage> for ListingImageResponseDto {
    fn from(img: ListingImage) -> Self {
        Self {
            id: img.id.0,
            image_url: img.image_url,
            position: img.position,
        }
    }
}

impl ListingResponseDto {
    pub fn from_listing(listing: Listing) -> Self {
        Self {
            id: listing.id.0,
            seller_id: listing.seller_id,
            title: listing.title,
            description: listing.description,
            price: listing.price,
            currency: listing.currency,
            category: listing.category,
            condition: listing.condition,
            status: listing.status,
            location_lat: listing.location_lat,
            location_lon: listing.location_lon,
            city: listing.city,
            images: listing.images.into_iter().map(ListingImageResponseDto::from).collect(),
            created_at: listing.created_at,
            updated_at: listing.updated_at,
        }
    }
}

impl ListingSummaryDto {
    pub fn from_listing(listing: &Listing) -> Self {
        let first_image_url = listing.images.first().map(|img| img.image_url.clone());
        Self {
            id: listing.id.0,
            seller_id: listing.seller_id,
            title: listing.title.clone(),
            price: listing.price,
            currency: listing.currency.clone(),
            category: listing.category.clone(),
            condition: listing.condition.clone(),
            status: listing.status.clone(),
            city: listing.city.clone(),
            first_image_url,
            image_count: listing.images.len() as i32,
            created_at: listing.created_at,
            updated_at: listing.updated_at,
        }
    }
}
