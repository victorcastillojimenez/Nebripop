use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use crate::adapters::rating_repository::RatingRepository;
use crate::errors::RatingError;
use crate::usecases::list_ratings_usecase;

use common::errors::AppError;
use common::pagination::PageRequest;

/// GET /users/:id/ratings
///
/// Retorna las valoraciones recibidas por un usuario (público).
/// - page: número de página (default: 0)
/// - per_page: resultados por página (default: 20, max: 100)
///
/// Errores:
/// - 404: usuario no encontrado
pub async fn list_ratings_handler(
    State(repo): State<RatingRepository>,
    Path(user_id): Path<Uuid>,
    Query(pagination): Query<PageRequest>,
) -> Result<Json<crate::dtos::RatingsListDto>, AppError> {
    let per_page = pagination.per_page.min(100).max(1);
    let page = pagination.page.max(0);

    let result = list_ratings_usecase::list_ratings_usecase(&repo, user_id, page, per_page)
        .await
        .map_err(|e| match e {
            RatingError::DatabaseError(msg) => {
                tracing::error!("Database error in list_ratings: {}", msg);
                AppError::Internal("Error interno del servidor".to_string())
            }
            _ => AppError::Internal("Error interno del servidor".to_string()),
        })?;

    Ok(Json(result))
}
