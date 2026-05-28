use uuid::Uuid;

use crate::errors::FavoriteError;
use crate::ports::FavoritePort;

pub async fn remove_favorite_usecase(
    repo: &dyn FavoritePort,
    user_id: Uuid,
    listing_id: Uuid,
) -> Result<(), FavoriteError> {
    repo.delete_favorite(user_id, listing_id).await
}
