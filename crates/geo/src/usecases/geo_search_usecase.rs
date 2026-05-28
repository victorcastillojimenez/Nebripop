use crate::adapters::geo_repository::GeoRepository;
use crate::dtos::{GeoSearchQuery, GeoSearchResponse};
use crate::errors::GeoError;
use crate::models::GeoPoint;

/// Radio máximo permitido: 50 km (50,000 metros).
const MAX_RADIUS_METERS: f64 = 50_000.0;

/// Límite máximo de resultados.
const MAX_LIMIT: i64 = 100;

/// Límite por defecto de resultados.
const DEFAULT_LIMIT: i64 = 20;

pub async fn geo_search_usecase(
    repo: &GeoRepository,
    query: GeoSearchQuery,
) -> Result<GeoSearchResponse, GeoError> {
    // 1. Validar coordenadas
    let point = GeoPoint::new(query.lat, query.lng);
    point.validate().map_err(|msg| GeoError::InvalidCoordinates(msg.to_string()))?;

    // 2. Validar radio máximo
    if query.radius > MAX_RADIUS_METERS as u32 {
        return Err(GeoError::RadiusExceeded);
    }

    // 3. Validar límite
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT as u32) as i64;
    let limit = limit.min(MAX_LIMIT).max(1);

    // 4. Ejecutar búsqueda
    let listings = repo.search_nearby(query.lat, query.lng, query.radius as f64, limit).await?;

    Ok(GeoSearchResponse::new(listings))
}
