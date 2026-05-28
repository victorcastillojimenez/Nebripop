use uuid::Uuid;

use crate::dtos::FavoritesListDto;
use crate::errors::FavoriteError;
use crate::ports::FavoritePort;

pub async fn list_favorites_usecase(
    repo: &dyn FavoritePort,
    user_id: Uuid,
    page: i64,
    per_page: i64,
) -> Result<FavoritesListDto, FavoriteError> {
    let offset = page * per_page;

    let data = repo.find_by_user_id(user_id, offset, per_page).await?;
    let total = repo.count_by_user_id(user_id).await?;

    Ok(FavoritesListDto::new(data, total))
}
