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

use search::adapters::meilisearch_adapter::MeiliSearchAdapter;
use search::ports::SearchEngine;

/// POST /listings
///
/// Creates a new listing for the authenticated user.
/// After creation, the listing is also indexed in MeiliSearch (best-effort).
///
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
    State(search_engine): State<Option<MeiliSearchAdapter>>,
    auth_user: AuthUser,
    Json(dto): Json<CreateListingDto>,
) -> Result<(StatusCode, Json<ListingResponseDto>), AppError> {
    // Validate DTO
    dto.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Pass search engine as a trait reference for best-effort indexing
    let engine_ref: Option<&dyn SearchEngine> =
        search_engine.as_ref().map(|e| e as &dyn SearchEngine);

    let result = create_listing_usecase::create_listing_usecase(
        &repo,
        engine_ref,
        auth_user.id,
        dto,
    )
    .await
    .map_err(map_listing_error)?;

    Ok((StatusCode::CREATED, Json(result)))
}
