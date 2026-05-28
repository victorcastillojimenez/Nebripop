use axum::extract::{Query, State};
use axum::Json;

use crate::adapters::favorite_repository::FavoriteRepository;
use crate::errors::FavoriteError;
use crate::usecases::list_favorites_usecase;

use common::auth::AuthUser;
use common::errors::AppError;
use common::pagination::PageRequest;

/// GET /users/me/favorites
///
/// Retorna los favoritos del usuario autenticado.
/// - page: número de página (default: 0)
/// - per_page: resultados por página (default: 20, max: 100)
///
/// Errores:
/// - 401: no autenticado
pub async fn list_favorites_handler(
    State(repo): State<FavoriteRepository>,
    auth_user: AuthUser,
    Query(pagination): Query<PageRequest>,
) -> Result<Json<crate::dtos::FavoritesListDto>, AppError> {
    let per_page = pagination.per_page.min(100).max(1);
    let page = pagination.page.max(0);

    let result =
        list_favorites_usecase::list_favorites_usecase(&repo, auth_user.id, page, per_page)
            .await
            .map_err(|e| match e {
                FavoriteError::DatabaseError(msg) => {
                    tracing::error!("Database error in list_favorites: {}", msg);
                    AppError::Internal("Error interno del servidor".to_string())
                }
                _ => AppError::Internal("Error interno del servidor".to_string()),
            })?;

    Ok(Json(result))
}
