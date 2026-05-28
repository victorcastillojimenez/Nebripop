use axum::routing::get;
use axum::Router;

use crate::adapters::geo_repository::GeoRepository;
use crate::handlers;

/// Retorna el router del módulo geo con sus rutas.
///
/// Rutas:
/// - GET /listings/nearby (público)
pub fn geo_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    GeoRepository: axum::extract::FromRef<S>,
{
    Router::new().route(
        "/listings/nearby",
        get(handlers::search_by_geo::search_by_geo_handler),
    )
}
