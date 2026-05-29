use async_trait::async_trait;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::errors::ListingError;
use crate::models::{Listing, ListingImage, PhysicalCondition};

/// Primary port (repository interface) for listing persistence.
/// Defined in the domain so usecases depend on this trait, not on infrastructure.
#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait ListingRepository: Send + Sync {
    /// Find a listing by its ID. Returns None if not found.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Listing>, ListingError>;

    /// Find all active listings with pagination, ordered by created_at DESC.
    async fn find_all_paginated(
        &self,
        page: i64,
        per_page: i64,
        category: Option<&str>,
    ) -> Result<(Vec<Listing>, i64), ListingError>;

    /// Find all listings by seller ID, ordered by created_at DESC.
    async fn find_by_seller(
        &self,
        seller_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Listing>, i64), ListingError>;

    /// Insert a new listing and return it with generated fields.
    async fn insert(
        &self,
        id: Uuid,
        seller_id: Uuid,
        title: &str,
        description: &str,
        price: Decimal,
        currency: &str,
        category: &str,
        condition: &PhysicalCondition,
        location_lat: f64,
        location_lon: f64,
        city: &str,
    ) -> Result<Listing, ListingError>;

    /// Update an existing listing. Only non-None fields will be updated.
    async fn update(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        price: Option<Decimal>,
        category: Option<&str>,
        condition: Option<&PhysicalCondition>,
        location_lat: Option<f64>,
        location_lon: Option<f64>,
        city: Option<&str>,
    ) -> Result<Listing, ListingError>;

    /// Soft delete a listing by setting status to 'deleted'.
    async fn soft_delete(&self, id: Uuid) -> Result<(), ListingError>;

    /// Mark a listing as sold.
    async fn mark_as_sold(&self, id: Uuid) -> Result<(), ListingError>;

    /// Count images for a listing.
    async fn count_images(&self, listing_id: Uuid) -> Result<i32, ListingError>;

    /// Insert a new listing image and return it.
    async fn insert_image(
        &self,
        id: Uuid,
        listing_id: Uuid,
        image_url: &str,
        position: i32,
    ) -> Result<ListingImage, ListingError>;

    /// Find all images for a listing.
    async fn find_images_by_listing(&self, listing_id: Uuid) -> Result<Vec<ListingImage>, ListingError>;

    /// Delete all images for a listing (by listing_id).
    async fn delete_images_by_listing(&self, listing_id: Uuid) -> Result<(), ListingError>;
}

/// Primary port for image storage operations (Cloudinary with local fallback).
#[async_trait]
pub trait ImageStorage: Send + Sync {
    /// Upload an image from raw bytes. Returns the public URL.
    async fn upload(&self, bytes: Vec<u8>, filename: &str, content_type: &str) -> Result<String, ListingError>;

    /// Delete an image by its URL/public ID.
    async fn delete(&self, url: &str) -> Result<(), ListingError>;

    /// Transform a base URL to an optimized version with Cloudinary transformations.
    fn get_optimized_url(&self, url: &str) -> String;
}
