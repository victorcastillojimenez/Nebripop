use async_trait::async_trait;
use uuid::Uuid;

use crate::dtos::FavoriteDto;
use crate::errors::FavoriteError;
use crate::models::Favorite;

/// Puerto primario para operaciones de favoritos.
#[async_trait]
pub trait FavoritePort: Send + Sync {
    /// Inserta un favorito de forma idempotente.
    /// Retorna el Favorite creado y un booleano indicando si ya existía.
    async fn insert_favorite(
        &self,
        id: Uuid,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> Result<(Favorite, bool), FavoriteError>;

    /// Elimina un favorito. Retorna NotFound si no existe.
    async fn delete_favorite(&self, user_id: Uuid, listing_id: Uuid) -> Result<(), FavoriteError>;

    /// Busca todos los favoritos de un usuario con datos del listing.
    async fn find_by_user_id(
        &self,
        user_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<FavoriteDto>, FavoriteError>;

    /// Cuenta los favoritos de un usuario.
    async fn count_by_user_id(&self, user_id: Uuid) -> Result<i64, FavoriteError>;

    /// Verifica si existe un favorito.
    async fn exists(&self, user_id: Uuid, listing_id: Uuid) -> Result<bool, FavoriteError>;
}
