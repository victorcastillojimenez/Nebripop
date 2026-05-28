use async_trait::async_trait;

use crate::errors::GeoError;
use crate::models::GeoListing;

/// Puerto primario para búsqueda geográfica de anuncios.
#[async_trait]
pub trait GeoPort: Send + Sync {
    /// Busca anuncios activos cercanos a una ubicación usando PostGIS.
    async fn search_nearby(
        &self,
        lat: f64,
        lng: f64,
        radius_m: f64,
        max_limit: i64,
    ) -> Result<Vec<GeoListing>, GeoError>;
}
