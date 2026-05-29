use uuid::Uuid;

use crate::dtos::ListingImageResponseDto;
use crate::errors::ListingError;
use crate::models::ListingStatus;
use crate::ports::{ImageStorage, ListingRepository};

/// Maximum number of images allowed per listing.
const MAX_IMAGES_PER_LISTING: i32 = 10;

pub async fn upload_image_usecase(
    repo: &dyn ListingRepository,
    image_storage: &dyn ImageStorage,
    listing_id: Uuid,
    user_id: Uuid,
    image_bytes: Vec<u8>,
    filename: &str,
    content_type: &str,
) -> Result<ListingImageResponseDto, ListingError> {
    // 1. Verify listing exists
    let existing = repo
        .find_by_id(listing_id)
        .await?
        .ok_or(ListingError::NotFound(listing_id))?;

    // 2. Verify ownership
    if existing.seller_id != user_id {
        return Err(ListingError::NotOwner(listing_id));
    }

    // 3. Verify listing is active
    if existing.status != ListingStatus::Active {
        return Err(ListingError::AlreadySold(listing_id));
    }

    // 4. Check image count limit
    let current_count = repo.count_images(listing_id).await?;
    if current_count >= MAX_IMAGES_PER_LISTING {
        return Err(ListingError::TooManyImages);
    }

    // 5. Upload image to storage
    let image_url = image_storage
        .upload(image_bytes, filename, content_type)
        .await?;

    // 6. Apply Cloudinary optimised URL
    let optimized_url = image_storage.get_optimized_url(&image_url);

    // 7. Persist image record
    let image_id = Uuid::new_v4();
    let image = repo
        .insert_image(image_id, listing_id, &optimized_url, current_count)
        .await?;

    Ok(ListingImageResponseDto::from(image))
}
