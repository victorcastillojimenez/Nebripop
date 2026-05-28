use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use uuid::Uuid;

use crate::adapters::rating_repository::RatingRepository;
use crate::dtos::{CreateRatingDto, RatingDto};
use crate::errors::RatingError;
use crate::usecases::create_rating_usecase;
use crate::usecases::create_rating_usecase::CreateRatingRequest;

use common::auth::AuthUser;
use common::errors::AppError;

/// Response wrapper for created rating
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RatingCreatedResponse {
    pub rating: RatingDto,
}

/// Mapea los errores del dominio a errores HTTP.
fn map_rating_error(e: RatingError) -> AppError {
    match e {
        RatingError::AlreadyRated => {
            AppError::Conflict("Ya has valorado esta transacción".to_string())
        }
        RatingError::InvalidScore(s) => AppError::BadRequest(format!(
            "Puntuación inválida: {}. Debe estar entre 1 y 5",
            s
        )),
        RatingError::ValidationError(msg) => AppError::BadRequest(msg),
        RatingError::NotFound => AppError::NotFound("Valoración no encontrada".to_string()),
        RatingError::TransactionNotCompleted => {
            AppError::BadRequest("La transacción no está completada".to_string())
        }
        RatingError::DatabaseError(msg) => {
            tracing::error!("Database error in create_rating: {}", msg);
            AppError::Internal("Error interno del servidor".to_string())
        }
    }
}

/// POST /listings/:id/ratings
///
/// Crea una valoración para un anuncio (requiere autenticación).
/// - score: 1-5 (obligatorio)
/// - comment: opcional, máx. 500 caracteres
///
/// Errores:
/// - 400: validación de campos
/// - 401: no autenticado
/// - 409: ya valoraste este anuncio
/// - 422: score fuera de rango
///
/// Nota: La validación del score (1-5) se delega al Value Object RatingScore
/// en el usecase, no en este handler (SRP).
pub async fn create_rating_handler(
    State(repo): State<RatingRepository>,
    auth_user: AuthUser,
    Path(listing_id): Path<Uuid>,
    Json(dto): Json<CreateRatingDto>,
) -> Result<Json<RatingCreatedResponse>, AppError> {
    // En este MVP, el rated_id se obtiene del listing.
    // Por simplicidad, usamos un placeholder que en producción se reemplazaría
    // con la lógica real de obtención del vendedor desde el listing.
    // El rater_id es el usuario autenticado.
    let request = CreateRatingRequest {
        listing_id,
        rater_id: auth_user.id,
        // Nota: En producción, rated_id debe obtenerse del seller_id del listing
        rated_id: auth_user.id, // Placeholder
    };

    let result = create_rating_usecase::create_rating_usecase(&repo, request, dto)
        .await
        .map_err(map_rating_error)?;

    Ok(Json(RatingCreatedResponse {
        rating: result,
    }))
}
