use uuid::Uuid;

use crate::errors::ListingError;
use crate::ports::{ImageStorage, ListingRepository};

pub async fn delete_listing_usecase(
    repo: &dyn ListingRepository,
    image_storage: &dyn ImageStorage,
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

    Ok(())
}
