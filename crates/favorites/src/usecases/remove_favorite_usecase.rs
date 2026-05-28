use uuid::Uuid;

use crate::adapters::favorite_repository::FavoriteRepository;
use crate::errors::FavoriteError;

pub async fn remove_favorite_usecase(
    repo: &FavoriteRepository,
    user_id: Uuid,
    listing_id: Uuid,
) -> Result<(), FavoriteError> {
    repo.delete_favorite(user_id, listing_id).await
}
