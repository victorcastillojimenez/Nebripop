use axum::extract::{Query, State};
use axum::Json;

use crate::adapters::listing_repository::ListingRepositoryImpl;
use crate::dtos::{ListingSummaryDto, PaginatedResponse, PaginationParams};
use crate::errors::map_listing_error;
use crate::ports::ListingRepository;

use common::errors::AppError;

/// GET /listings
///
/// Returns a paginated list of active listings.
/// - page: page number (default: 0)
/// - per_page: items per page (default: 20, max: 100)
/// - category: optional category filter
///
/// Public endpoint — no authentication required.
pub async fn list_listings_handler(
    State(repo): State<ListingRepositoryImpl>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<ListingSummaryDto>>, AppError> {
    let per_page = params.per_page.clamp(1, 100);
    let page = params.page.max(0);
    let category = params.category.as_deref();
    let condition = params.condition.as_deref();

    let (listings, total) = repo
        .find_all_paginated(page, per_page, category, condition)
        .await
        .map_err(map_listing_error)?;

    let data: Vec<ListingSummaryDto> = listings
        .iter()
        .map(ListingSummaryDto::from_listing)
        .collect();

    Ok(Json(PaginatedResponse::new(data, page, per_page, total)))
}
