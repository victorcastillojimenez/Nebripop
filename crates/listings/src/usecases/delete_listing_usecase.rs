use uuid::Uuid;

use crate::errors::ListingError;
use crate::ports::{ImageStorage, ListingRepository};

use search::ports::SearchEngine;

/// Soft-delete a listing and remove it from the MeiliSearch index.
///
/// # Errors
///
/// Returns `ListingError::NotFound` if the listing does not exist,
/// or `ListingError::NotOwner` if the user is not the owner.
///
/// # Search removal
///
/// After successful soft delete in PostgreSQL, the listing document is removed
/// from MeiliSearch. If removal fails, the error is logged but the operation
/// is **not** reverted (best-effort).
pub async fn delete_listing_usecase(
    repo: &dyn ListingRepository,
    image_storage: &dyn ImageStorage,
    search_engine: Option<&dyn SearchEngine>,
    listing_id: Uuid,
    user_id: Uuid,
) -> Result<(), ListingError> {
    // 1. Verify listing exists
    let existing = repo
        .find_by_id(listing_id)
        .await?
        .ok_or(ListingError::NotFound(listing_id))?;

    // 2. Verify ownership (only owner can delete)
    if existing.seller_id != user_id {
        return Err(ListingError::NotOwner(listing_id));
    }

    // 3. Delete images from storage (fire-and-forget to avoid blocking)
    let image_urls: Vec<String> = existing
        .images
        .iter()
        .map(|img| img.image_url.clone())
        .collect();
    for url in &image_urls {
        // Best-effort deletion; ignore errors to not block the soft delete
        if let Err(e) = image_storage.delete(url).await {
            tracing::warn!("Failed to delete image {}: {}", url, e);
        }
    }

    // 4. Soft delete the listing
    repo.soft_delete(listing_id).await?;

    // 5. Remove listing from MeiliSearch index (best-effort, non-blocking).
    // If the search engine is unavailable or returns an error,
    // log a warning but do NOT fail the delete operation.
    if let Some(engine) = search_engine {
        if let Err(e) = engine.remove_listing(listing_id).await {
            tracing::warn!(
                error = %e,
                listing_id = %listing_id.to_string(),
                "MeiliSearch removal failed after listing deletion"
            );
        }
    }

    Ok(())
}
