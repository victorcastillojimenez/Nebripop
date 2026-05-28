use uuid::Uuid;

use crate::dtos::AddFavoriteResponse;
use crate::errors::FavoriteError;
use crate::ports::FavoritePort;

pub async fn add_favorite_usecase(
    repo: &dyn FavoritePort,
    user_id: Uuid,
    listing_id: Uuid,
) -> Result<AddFavoriteResponse, FavoriteError> {
    let id = Uuid::new_v4();
    let (_fav, already_existed) = repo.insert_favorite(id, user_id, listing_id).await?;

    Ok(AddFavoriteResponse::new(!already_existed, already_existed))
}
