use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::adapters::listing_repository::ListingRepositoryImpl;
use crate::dtos::ListingResponseDto;
use crate::errors::map_listing_error;
use crate::usecases::get_listing_usecase;

use common::errors::AppError;

/// GET /listings/:id
///
/// Returns full listing details including images.
///
/// Public endpoint — no authentication required.
/// Errors: 404 if listing not found.
pub async fn get_listing_handler(
    State(repo): State<ListingRepositoryImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ListingResponseDto>, AppError> {
    let result = get_listing_usecase::get_listing_usecase(&repo, id)
        .await
        .map_err(map_listing_error)?;

    Ok(Json(result))
}
