use rust_decimal::Decimal;
use uuid::Uuid;

use crate::dtos::{ListingResponseDto, UpdateListingDto};
use crate::errors::ListingError;
use crate::models::ListingStatus;
use crate::ports::ListingRepository;

pub async fn update_listing_usecase(
    repo: &dyn ListingRepository,
    listing_id: Uuid,
    user_id: Uuid,
    dto: UpdateListingDto,
) -> Result<ListingResponseDto, ListingError> {
    // 1. Verify listing exists and get current state
    let existing = repo
        .find_by_id(listing_id)
        .await?
        .ok_or(ListingError::NotFound(listing_id))?;

    // 2. Verify ownership
    if existing.seller_id != user_id {
        return Err(ListingError::NotOwner(listing_id));
    }

    // 3. Verify listing is active (cannot edit sold or deleted)
    if existing.status != ListingStatus::Active {
        return Err(ListingError::AlreadySold(listing_id));
    }

    // 4. Validate price if provided
    if let Some(price) = dto.price {
        if price <= Decimal::ZERO {
            return Err(ListingError::InvalidInput(
                "El precio debe ser mayor que 0".to_string(),
            ));
        }
        if price > Decimal::new(99999999, 2) {
            return Err(ListingError::InvalidInput(
                "El precio no puede exceder 999999.99".to_string(),
            ));
        }
    }

    // 5. Validate coordinates if provided
    if let Some(lat) = dto.location_lat {
        if lat < -90.0 || lat > 90.0 {
            return Err(ListingError::InvalidInput(
                "Latitud inválida: debe estar entre -90 y 90".to_string(),
            ));
        }
    }
    if let Some(lon) = dto.location_lon {
        if lon < -180.0 || lon > 180.0 {
            return Err(ListingError::InvalidInput(
                "Longitud inválida: debe estar entre -180 y 180".to_string(),
            ));
        }
    }

    // 6. Apply updates
    let listing = repo
        .update(
            listing_id,
            dto.title.as_deref(),
            dto.description.as_deref(),
            dto.price,
            dto.category.as_deref(),
            dto.condition.as_ref(),
            dto.location_lat,
            dto.location_lon,
            dto.city.as_deref(),
        )
        .await?;

    Ok(ListingResponseDto::from_listing(listing))
}
