use uuid::Uuid;

use crate::errors::SearchError;
use crate::models::ListingDoc;
use crate::ports::SearchEngine;

/// Index a single listing document in the search engine.
pub async fn index_listing(
    engine: &dyn SearchEngine,
    doc: &ListingDoc,
) -> Result<(), SearchError> {
    engine.index_listing(doc).await
}

/// Remove a single listing from the search engine by its ID.
pub async fn remove_listing(
    engine: &dyn SearchEngine,
    listing_id: Uuid,
) -> Result<(), SearchError> {
    engine.remove_listing(listing_id).await
}
