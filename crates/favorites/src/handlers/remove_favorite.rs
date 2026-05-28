use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::adapters::favorite_repository::FavoriteRepository;
use crate::errors::FavoriteError;
use crate::usecases::remove_favorite_usecase;

use common::auth::AuthUser;
use common::errors::AppError;

/// DELETE /listings/:id/favorites
///
/// Elimina un anuncio de favoritos (requiere autenticación).
///
/// Errores:
/// - 401: no autenticado
/// - 404: favorito no encontrado
pub async fn remove_favorite_handler(
    State(repo): State<FavoriteRepository>,
    auth_user: AuthUser,
    Path(listing_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    remove_favorite_usecase::remove_favorite_usecase(&repo, auth_user.id, listing_id)
        .await
        .map_err(|e| match e {
            FavoriteError::NotFound => {
                AppError::NotFound("Favorito no encontrado".to_string())
            }
            FavoriteError::DatabaseError(msg) => {
                tracing::error!("Database error in remove_favorite: {}", msg);
                AppError::Internal("Error interno del servidor".to_string())
            }
            _ => AppError::Internal("Error interno del servidor".to_string()),
        })?;

    Ok(StatusCode::NO_CONTENT)
}
