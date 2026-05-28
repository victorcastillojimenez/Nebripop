use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use validator::Validate;

use crate::adapters::listing_repository::ListingRepositoryImpl;
use crate::dtos::{CreateListingDto, ListingResponseDto};
use crate::errors::map_listing_error;
use crate::usecases::create_listing_usecase;

use common::auth::AuthUser;
use common::errors::AppError;

/// POST /listings
///
/// Creates a new listing for the authenticated user.
/// - title: required, 3-100 characters
/// - description: required, max 2000 characters
/// - price: required, must be > 0
/// - category: required
/// - condition: required (new, like_new, used)
/// - locationLat: required
/// - locationLon: required
/// - city: required
///
/// Authentication: required (JWT Bearer token)
pub async fn create_listing_handler(
    State(repo): State<ListingRepositoryImpl>,
    auth_user: AuthUser,
    Json(dto): Json<CreateListingDto>,
) -> Result<(StatusCode, Json<ListingResponseDto>), AppError> {
    // Validate DTO
    dto.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let result = create_listing_usecase::create_listing_usecase(&repo, auth_user.id, dto)
        .await
        .map_err(map_listing_error)?;

    Ok((StatusCode::CREATED, Json(result)))
}
