use async_trait::async_trait;
use uuid::Uuid;

use crate::errors::RatingError;
use crate::models::Rating;

/// Puerto primario para operaciones de valoraciones.
/// Definido en el dominio para invertir la dependencia (DIP).
#[async_trait]
pub trait RatingPort: Send + Sync {
    /// Inserta una nueva valoración.
    async fn insert_rating(
        &self,
        id: Uuid,
        listing_id: Uuid,
        rater_id: Uuid,
        rated_id: Uuid,
        score: i16,
        comment: Option<&str>,
    ) -> Result<Rating, RatingError>;

    /// Busca valoraciones recibidas por un usuario con paginación.
    async fn find_by_user_id(
        &self,
        user_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Rating>, RatingError>;

    /// Calcula el promedio y total de valoraciones de un usuario.
    async fn calculate_average(&self, user_id: Uuid) -> Result<(f64, i64), RatingError>;

    /// Cuenta el total de valoraciones de un usuario.
    async fn count_by_user_id(&self, user_id: Uuid) -> Result<i64, RatingError>;

    /// Verifica si ya existe una valoración del rater para este listing.
    async fn exists(&self, listing_id: Uuid, rater_id: Uuid) -> Result<bool, RatingError>;
}
