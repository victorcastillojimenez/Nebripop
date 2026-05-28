use axum::extract::{Query, State};
use axum::Json;

use crate::adapters::geo_repository::GeoRepository;
use crate::dtos::GeoSearchQuery;
use crate::errors::GeoError;
use crate::usecases::geo_search_usecase;

use common::errors::AppError;

/// GET /listings/nearby
///
/// Busca anuncios activos cercanos a una ubicación (público).
///
/// Query params:
/// - lat: latitud del punto de origen
/// - lng: longitud del punto de origen
/// - radius: radio de búsqueda en metros (máx. 50000)
/// - limit: límite de resultados (máx. 100, default: 20)
///
/// Errores:
/// - 400: coordenadas inválidas o radio excedido
pub async fn search_by_geo_handler(
    State(repo): State<GeoRepository>,
    Query(query): Query<GeoSearchQuery>,
) -> Result<Json<crate::dtos::GeoSearchResponse>, AppError> {
    let result = geo_search_usecase::geo_search_usecase(&repo, query)
        .await
        .map_err(|e| match e {
            GeoError::InvalidCoordinates(msg) => {
                AppError::BadRequest(format!("Coordenadas inválidas: {}", msg))
            }
            GeoError::RadiusExceeded => {
                AppError::BadRequest(
                    "Radio excedido: máximo permitido 50000 metros (50 km)".to_string(),
                )
            }
            GeoError::LimitExceeded => {
                AppError::BadRequest(
                    "Límite excedido: máximo permitido 100 resultados".to_string(),
                )
            }
            GeoError::DatabaseError(msg) => {
                tracing::error!("Database error in search_by_geo: {}", msg);
                AppError::Internal("Error interno del servidor".to_string())
            }
        })?;

    Ok(Json(result))
}
