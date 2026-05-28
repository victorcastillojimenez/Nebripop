use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::models::Rating;

/// DTO para crear una nueva valoración.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateRatingDto {
    /// Puntuación del 1 al 5.
    pub score: i16,

    /// Comentario opcional (máx. 500 caracteres).
    #[validate(length(
        max = 500,
        message = "El comentario no puede exceder los 500 caracteres"
    ))]
    pub comment: Option<String>,
}

/// DTO de respuesta para una valoración.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RatingDto {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub rater_id: Uuid,
    pub rated_id: Uuid,
    pub score: i16,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Rating> for RatingDto {
    fn from(r: Rating) -> Self {
        Self {
            id: r.id,
            listing_id: r.listing_id,
            rater_id: r.rater_id,
            rated_id: r.rated_id,
            score: r.score,
            comment: r.comment,
            created_at: r.created_at,
        }
    }
}

/// DTO de respuesta para listado paginado de valoraciones.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RatingsListDto {
    pub data: Vec<RatingDto>,
    pub total: i64,
    pub average_score: f64,
}

impl RatingsListDto {
    pub fn new(data: Vec<Rating>, total: i64, average_score: f64) -> Self {
        Self {
            data: data.into_iter().map(RatingDto::from).collect(),
            total,
            average_score,
        }
    }
}
