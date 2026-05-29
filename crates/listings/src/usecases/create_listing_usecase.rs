use rust_decimal::Decimal;
use uuid::Uuid;

use crate::dtos::{CreateListingDto, ListingResponseDto};
use crate::errors::ListingError;
use crate::ports::ListingRepository;

pub async fn create_listing_usecase(
    repo: &dyn ListingRepository,
    seller_id: Uuid,
    dto: CreateListingDto,
) -> Result<ListingResponseDto, ListingError> {
    // Validate price
    if dto.price <= Decimal::ZERO {
        return Err(ListingError::InvalidInput(
            "El precio debe ser mayor que 0".to_string(),
        ));
    }
    if dto.price > Decimal::new(99999999, 2) {
        return Err(ListingError::InvalidInput(
            "El precio no puede exceder 999999.99".to_string(),
        ));
    }

    // Validate coordinates
    if dto.location_lat < -90.0 || dto.location_lat > 90.0 {
        return Err(ListingError::InvalidInput(
            "Latitud inválida: debe estar entre -90 y 90".to_string(),
        ));
    }
    if dto.location_lon < -180.0 || dto.location_lon > 180.0 {
        return Err(ListingError::InvalidInput(
            "Longitud inválida: debe estar entre -180 y 180".to_string(),
        ));
    }

    let id = Uuid::new_v4();
    let listing = repo
        .insert(
            id,
            seller_id,
            &dto.title,
            &dto.description,
            dto.price,
            "eur",
            &dto.category,
            &dto.condition,
            dto.location_lat,
            dto.location_lon,
            &dto.city,
        )
        .await?;

    Ok(ListingResponseDto::from_listing(listing))
}
