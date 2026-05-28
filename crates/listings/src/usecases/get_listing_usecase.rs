use uuid::Uuid;

use crate::dtos::ListingResponseDto;
use crate::errors::ListingError;
use crate::ports::ListingRepository;

pub async fn get_listing_usecase(
    repo: &dyn ListingRepository,
    id: Uuid,
) -> Result<ListingResponseDto, ListingError> {
    let listing = repo
        .find_by_id(id)
        .await?
        .ok_or(ListingError::NotFound(id))?;

    Ok(ListingResponseDto::from_listing(listing))
}
