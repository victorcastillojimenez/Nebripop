use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::adapters::listing_repository::ListingRepositoryImpl;
use crate::dtos::{ListingResponseDto, UpdateListingDto};
use crate::errors::map_listing_error;
use crate::usecases::update_listing_usecase;

use common::auth::AuthUser;
use common::errors::AppError;

use search::adapters::meilisearch_adapter::MeiliSearchAdapter;
use search::ports::SearchEngine;

/// PUT /listings/:id
///
/// Updates an existing listing. Only the owner can update.
/// All fields are optional (PATCH semantics).
/// After update, the listing is re-indexed in MeiliSearch (best-effort).
///
/// Authentication: required (JWT Bearer token — must be the owner)
/// Errors: 403 if not the owner, 404 if not found.
pub async fn update_listing_handler(
    State(repo): State<ListingRepositoryImpl>,
    State(search_engine): State<Option<MeiliSearchAdapter>>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateListingDto>,
) -> Result<Json<ListingResponseDto>, AppError> {
    // Pass search engine as a trait reference for best-effort re-indexing
    let engine_ref: Option<&dyn SearchEngine> =
        search_engine.as_ref().map(|e| e as &dyn SearchEngine);

    let result = update_listing_usecase::update_listing_usecase(
        &repo,
        engine_ref,
        id,
        auth_user.id,
        dto,
    )
    .await
    .map_err(map_listing_error)?;

    Ok(Json(result))
}
