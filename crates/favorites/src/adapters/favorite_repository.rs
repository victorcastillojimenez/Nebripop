use sqlx::PgPool;
use uuid::Uuid;

use crate::dtos::FavoriteDto;
use crate::errors::FavoriteError;
use crate::models::Favorite;

#[derive(Debug, Clone)]
pub struct FavoriteRepository {
    pool: PgPool,
}

impl FavoriteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Inserta un favorito de forma idempotente.
    /// Retorna el Favorite creado y un booleano indicando si ya existía.
    pub async fn insert_favorite(
        &self,
        id: Uuid,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> Result<(Favorite, bool), FavoriteError> {
        // ON CONFLICT DO NOTHING para idempotencia
        let result = sqlx::query_as::<_, Favorite>(
            r#"INSERT INTO favorites (id, user_id, listing_id)
               VALUES ($1, $2, $3)
               ON CONFLICT (user_id, listing_id) DO NOTHING
               RETURNING id, user_id, listing_id, created_at"#,
        )
        .bind(id)
        .bind(user_id)
        .bind(listing_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in insert_favorite: {}", e);
            FavoriteError::DatabaseError(e.to_string())
        })?;

        match result {
            Some(fav) => Ok((fav, false)), // Se insertó nuevo
            None => {
                // Ya existía — recuperarlo
                let existing = sqlx::query_as::<_, Favorite>(
                    r#"SELECT id, user_id, listing_id, created_at
                       FROM favorites
                       WHERE user_id = $1 AND listing_id = $2"#,
                )
                .bind(user_id)
                .bind(listing_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    tracing::error!("Database error in insert_favorite (fetch existing): {}", e);
                    FavoriteError::DatabaseError(e.to_string())
                })?;

                Ok((existing, true)) // Ya existía
            }
        }
    }

    /// Elimina un favorito. Retorna NotFound si no existe.
    pub async fn delete_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> Result<(), FavoriteError> {
        let result = sqlx::query(
            r#"DELETE FROM favorites WHERE user_id = $1 AND listing_id = $2"#,
        )
        .bind(user_id)
        .bind(listing_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in delete_favorite: {}", e);
            FavoriteError::DatabaseError(e.to_string())
        })?;

        if result.rows_affected() == 0 {
            return Err(FavoriteError::NotFound);
        }

        Ok(())
    }

    /// Busca todos los favoritos de un usuario con datos del listing.
    pub async fn find_by_user_id(
        &self,
        user_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<FavoriteDto>, FavoriteError> {
        let rows = sqlx::query_as::<_, FavoriteDtoRaw>(
            r#"SELECT
                f.id,
                f.user_id,
                f.listing_id,
                l.title AS listing_title,
                l.price AS listing_price,
                (SELECT li.image_url FROM listing_images li WHERE li.listing_id = l.id ORDER BY li.position ASC LIMIT 1) AS listing_image_url,
                l.city AS listing_city,
                f.created_at
               FROM favorites f
               LEFT JOIN listings l ON f.listing_id = l.id
               WHERE f.user_id = $1
               ORDER BY f.created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in find_by_user_id: {}", e);
            FavoriteError::DatabaseError(e.to_string())
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Cuenta los favoritos de un usuario.
    pub async fn count_by_user_id(&self, user_id: Uuid) -> Result<i64, FavoriteError> {
        let count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)::int8 FROM favorites WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in count_by_user_id: {}", e);
            FavoriteError::DatabaseError(e.to_string())
        })?;

        Ok(count.0)
    }

    /// Verifica si existe un favorito.
    pub async fn exists(&self, user_id: Uuid, listing_id: Uuid) -> Result<bool, FavoriteError> {
        let row: (bool,) = sqlx::query_as(
            r#"SELECT EXISTS(SELECT 1 FROM favorites WHERE user_id = $1 AND listing_id = $2)"#,
        )
        .bind(user_id)
        .bind(listing_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in exists: {}", e);
            FavoriteError::DatabaseError(e.to_string())
        })?;

        Ok(row.0)
    }

    /// Obtiene el pool (para uso en transacciones desde usecases).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Struct intermedio para mapear el resultado del JOIN con listing.
#[derive(Debug, sqlx::FromRow)]
struct FavoriteDtoRaw {
    pub id: Uuid,
    pub user_id: Uuid,
    pub listing_id: Uuid,
    pub listing_title: Option<String>,
    pub listing_price: Option<rust_decimal::Decimal>,
    pub listing_image_url: Option<String>,
    pub listing_city: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<FavoriteDtoRaw> for FavoriteDto {
    fn from(r: FavoriteDtoRaw) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            listing_id: r.listing_id,
            listing_title: r.listing_title,
            listing_price: r.listing_price,
            listing_image_url: r.listing_image_url,
            listing_city: r.listing_city,
            created_at: r.created_at,
        }
    }
}
