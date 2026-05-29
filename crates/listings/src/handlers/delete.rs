use axum::extract::{Path, State};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::adapters::cloudinary::ImageStorageImpl;
use crate::adapters::listing_repository::ListingRepositoryImpl;
use crate::errors::map_listing_error;
use crate::usecases::delete_listing_usecase;

use common::auth::AuthUser;
use common::errors::AppError;

use search::adapters::meilisearch_adapter::MeiliSearchAdapter;
use search::ports::SearchEngine;

/// DELETE /listings/:id
///
/// Soft-deletes a listing (status = 'deleted'). Only the owner can delete.
/// After deletion, the listing is also removed from the MeiliSearch index
/// (best-effort, non-blocking).
///
/// Authentication: required (JWT Bearer token — must be the owner)
/// Errors: 403 if not the owner, 404 if not found.
/// Response: 204 No Content on success.
pub async fn delete_listing_handler(
    State(repo): State<ListingRepositoryImpl>,
    State(image_storage): State<ImageStorageImpl>,
    State(search_engine): State<Option<MeiliSearchAdapter>>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    // Pass search engine as a trait reference for best-effort removal
    let engine_ref: Option<&dyn SearchEngine> =
        search_engine.as_ref().map(|e| e as &dyn SearchEngine);

    delete_listing_usecase::delete_listing_usecase(
        &repo,
        &image_storage,
        engine_ref,
        id,
        auth_user.id,
    )
    .await
    .map_err(map_listing_error)?;

    Ok(StatusCode::NO_CONTENT)
}
