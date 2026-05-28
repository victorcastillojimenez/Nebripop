use uuid::Uuid;

use crate::adapters::rating_repository::RatingRepository;
use crate::dtos::{CreateRatingDto, RatingDto};
use crate::errors::RatingError;
use crate::models::RatingScore;

pub struct CreateRatingRequest {
    pub listing_id: Uuid,
    pub rater_id: Uuid,
    pub rated_id: Uuid,
}

pub async fn create_rating_usecase(
    repo: &RatingRepository,
    req: CreateRatingRequest,
    dto: CreateRatingDto,
) -> Result<RatingDto, RatingError> {
    // 1. Validar puntuación (1-5)
    let _score = RatingScore::new(dto.score)?;

    // 2. Verificar que no exista ya una valoración del mismo rater para este listing
    let already_exists = repo.exists(req.listing_id, req.rater_id).await?;
    if already_exists {
        return Err(RatingError::AlreadyRated);
    }

    // 3. Validar longitud del comentario
    if let Some(ref comment) = dto.comment {
        if comment.len() > 500 {
            return Err(RatingError::ValidationError(
                "El comentario no puede exceder los 500 caracteres".to_string(),
            ));
        }
    }

    // 4. Insertar la valoración
    let id = Uuid::new_v4();
    let rating = repo
        .insert_rating(
            id,
            req.listing_id,
            req.rater_id,
            req.rated_id,
            dto.score,
            dto.comment.as_deref(),
        )
        .await?;

    Ok(RatingDto::from(rating))
}
