use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::adapters::user_repository::UserRepository;
use crate::dtos::PublicProfileDto;
use crate::errors::UserError;
use crate::models::PublicProfile;

use common::errors::AppError;

/// GET /users/:id
/// Returns the public profile of a user (never includes email or password_hash)
pub async fn get_profile_handler(
    State(repo): State<UserRepository>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<PublicProfileDto>, AppError> {
    // Find user
    let user = repo
        .find_by_id(user_id)
        .await
        .map_err(|e| match e {
            UserError::DatabaseError(msg) => {
                tracing::error!("Database error in get_profile: {}", msg);
                AppError::Internal("Error interno del servidor".to_string())
            }
            _ => AppError::Internal("Error interno del servidor".to_string()),
        })?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".to_string()))?;

    // Build public profile (never expose email or password_hash)
    let profile = PublicProfile {
        id: user.id,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        rating_avg: user.rating_avg.map(|d| {
            let f: f64 = d.to_string().parse().unwrap_or(0.0);
            f
        }).unwrap_or(0.0),
        total_ratings: user.total_ratings,
        created_at: user.created_at,
    };

    Ok(Json(PublicProfileDto::from(profile)))
}
