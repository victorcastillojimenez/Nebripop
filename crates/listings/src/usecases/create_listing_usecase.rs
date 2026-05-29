use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::dtos::{CreateListingDto, ListingResponseDto};
use crate::errors::ListingError;
use crate::models::Listing;
use crate::ports::ListingRepository;

use search::models::{Geo, ListingDoc};
use search::ports::SearchEngine;

/// Create a new listing and optionally index it in MeiliSearch.
///
/// # Errors
///
/// Returns `ListingError::InvalidInput` if validation fails,
/// or `ListingError::Database` if the database operation fails.
///
/// # Search indexing
///
/// After successful DB insert, the listing is indexed in MeiliSearch
/// via the `SearchEngine` trait. If indexing fails, the error is logged
/// but the main operation is **not** rolled back (fire-and-forget).
pub async fn create_listing_usecase(
    repo: &dyn ListingRepository,
    search_engine: Option<&dyn SearchEngine>,
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

    // Fire-and-forget indexing in MeiliSearch.
    // If the search engine is unavailable or returns an error,
    // log a warning but do NOT fail the create operation.
    if let Some(engine) = search_engine {
        let doc = listing_to_doc(&listing);
        if let Err(e) = engine.index_listing(&doc).await {
            tracing::warn!(
                error = %e,
                listing_id = %doc.id,
                "MeiliSearch index failed after listing creation"
            );
        }
    }

    Ok(ListingResponseDto::from_listing(listing))
}

/// Convert a domain `Listing` into a `ListingDoc` for MeiliSearch indexing.
fn listing_to_doc(listing: &Listing) -> ListingDoc {
    ListingDoc {
        id: listing.id.0.to_string(),
        title: listing.title.clone(),
        description: listing.description.clone(),
        price: listing.price.to_f64().unwrap_or(0.0),
        currency: listing.currency.clone(),
        category: listing.category.clone(),
        condition: listing.condition.as_str().to_string(),
        status: listing.status.as_str().to_string(),
        city: listing.city.clone(),
        _geo: Some(Geo {
            lat: listing.location_lat,
            lng: listing.location_lon,
        }),
        created_at: listing.created_at.timestamp(),
        image_url: listing.images.first().map(|img| img.image_url.clone()),
    }
}
