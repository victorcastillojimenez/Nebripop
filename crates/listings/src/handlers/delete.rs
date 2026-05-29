use axum::extract::{Path, State};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::adapters::cloudinary::ImageStorageImpl;
use crate::adapters::listing_repository::ListingRepositoryImpl;
use crate::errors::map_listing_error;
use crate::usecases::delete_listing_usecase;

use common::auth::AuthUser;
use common::errors::AppError;

/// DELETE /listings/:id
///
/// Soft-deletes a listing (status = 'deleted'). Only the owner can delete.
///
/// Authentication: required (JWT Bearer token — must be the owner)
/// Errors: 403 if not the owner, 404 if not found.
/// Response: 204 No Content on success.
pub async fn delete_listing_handler(
    State(repo): State<ListingRepositoryImpl>,
    State(image_storage): State<ImageStorageImpl>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    delete_listing_usecase::delete_listing_usecase(&repo, &image_storage, id, auth_user.id)
        .await
        .map_err(map_listing_error)?;

    Ok(StatusCode::NO_CONTENT)
}
