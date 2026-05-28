use uuid::Uuid;

use crate::adapters::rating_repository::RatingRepository;
use crate::dtos::RatingsListDto;
use crate::errors::RatingError;

pub async fn list_ratings_usecase(
    repo: &RatingRepository,
    user_id: Uuid,
    page: i64,
    per_page: i64,
) -> Result<RatingsListDto, RatingError> {
    let offset = page * per_page;

    let ratings = repo.find_by_user_id(user_id, offset, per_page).await?;
    let (average_score, total) = repo.calculate_average(user_id).await?;

    Ok(RatingsListDto::new(ratings, total, average_score))
}
