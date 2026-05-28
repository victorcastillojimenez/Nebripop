use axum::routing::{delete, get, post};
use axum::Router;

use crate::adapters::favorite_repository::FavoriteRepository;
use crate::handlers;

/// Retorna el router del módulo favorites con sus rutas.
///
/// Rutas:
/// - POST   /listings/:id/favorites (autenticado)
/// - DELETE /listings/:id/favorites (autenticado)
/// - GET    /users/me/favorites     (autenticado)
pub fn favorites_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    FavoriteRepository: axum::extract::FromRef<S>,
    String: axum::extract::FromRef<S>,
{
    Router::new()
        .route(
            "/listings/:id/favorites",
            post(handlers::add_favorite::add_favorite_handler),
        )
        .route(
            "/listings/:id/favorites",
            delete(handlers::remove_favorite::remove_favorite_handler),
        )
        .route(
            "/users/me/favorites",
            get(handlers::list_favorites::list_favorites_handler),
        )
}
