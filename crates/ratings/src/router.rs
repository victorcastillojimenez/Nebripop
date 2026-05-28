use axum::routing::{get, post};
use axum::Router;

use crate::adapters::rating_repository::RatingRepository;
use crate::handlers;

/// Retorna el router del módulo ratings con sus rutas.
///
/// Rutas:
/// - POST /listings/:id/ratings (autenticado)
/// - GET  /users/:id/ratings (público)
pub fn ratings_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    RatingRepository: axum::extract::FromRef<S>,
    String: axum::extract::FromRef<S>,
{
    Router::new()
        .route(
            "/listings/:id/ratings",
            post(handlers::create_rating::create_rating_handler),
        )
        .route(
            "/users/:id/ratings",
            get(handlers::list_ratings::list_ratings_handler),
        )
}
