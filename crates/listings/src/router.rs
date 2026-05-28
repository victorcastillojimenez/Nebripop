use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::adapters::cloudinary::ImageStorageImpl;
use crate::adapters::listing_repository::ListingRepositoryImpl;
use crate::handlers;

/// Build the listings router with all listing endpoints.
///
/// Public routes (no auth required):
/// - GET  /listings       — List active listings (paginated)
/// - GET  /listings/:id   — Get listing detail
///
/// Protected routes (JWT required):
/// - POST   /listings            — Create listing
/// - PUT    /listings/:id        — Update listing (owner only)
/// - DELETE /listings/:id        — Soft-delete listing (owner only)
/// - POST   /listings/:id/images — Upload image (owner only)
pub fn listings_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    ListingRepositoryImpl: axum::extract::FromRef<S>,
    ImageStorageImpl: axum::extract::FromRef<S>,
    String: axum::extract::FromRef<S>,
{
    Router::new()
        // Public routes
        .route("/listings", get(handlers::list::list_listings_handler))
        .route(
            "/listings/:id",
            get(handlers::get_by_id::get_listing_handler),
        )
        // Protected routes
        .route("/listings", post(handlers::create::create_listing_handler))
        .route(
            "/listings/:id",
            put(handlers::update::update_listing_handler),
        )
        .route(
            "/listings/:id",
            delete(handlers::delete::delete_listing_handler),
        )
        .route(
            "/listings/:id/images",
            post(handlers::upload_image::upload_image_handler),
        )
}
