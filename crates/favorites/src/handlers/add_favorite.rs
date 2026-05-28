use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::adapters::favorite_repository::FavoriteRepository;
use crate::errors::FavoriteError;
use crate::usecases::add_favorite_usecase;

use common::auth::AuthUser;
use common::errors::AppError;

/// POST /listings/:id/favorites
///
/// Añade un anuncio a favoritos (requiere autenticación).
/// Es idempotente: si ya existe, retorna 200 en lugar de error.
///
/// Errores:
/// - 401: no autenticado
/// - 404: anuncio no encontrado
pub async fn add_favorite_handler(
    State(repo): State<FavoriteRepository>,
    auth_user: AuthUser,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<crate::dtos::AddFavoriteResponse>, AppError> {
    let result = add_favorite_usecase::add_favorite_usecase(&repo, auth_user.id, listing_id)
        .await
        .map_err(|e| match e {
            FavoriteError::DatabaseError(msg) => {
                tracing::error!("Database error in add_favorite: {}", msg);
                AppError::Internal("Error interno del servidor".to_string())
            }
            _ => AppError::Internal("Error interno del servidor".to_string()),
        })?;

    Ok(Json(result))
}
