use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::RatingError;
use crate::models::Rating;
use crate::ports::RatingPort;

/// Fila de base de datos con sqlx::FromRow.
/// Privada al adaptador; se convierte al dominio sin acoplar la entidad.
#[derive(Debug, sqlx::FromRow)]
struct RatingRow {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub rater_id: Uuid,
    pub rated_id: Uuid,
    pub score: i16,
    pub comment: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<RatingRow> for Rating {
    fn from(r: RatingRow) -> Self {
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

#[derive(Debug, Clone)]
pub struct RatingRepository {
    pool: PgPool,
}

impl RatingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RatingPort for RatingRepository {
    async fn insert_rating(
        &self,
        id: Uuid,
        listing_id: Uuid,
        rater_id: Uuid,
        rated_id: Uuid,
        score: i16,
        comment: Option<&str>,
    ) -> Result<Rating, RatingError> {
        let rating = sqlx::query_as::<_, RatingRow>(
            r#"INSERT INTO ratings (id, listing_id, rater_id, rated_id, score, comment)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, listing_id, rater_id, rated_id, score, comment, created_at"#,
        )
        .bind(id)
        .bind(listing_id)
        .bind(rater_id)
        .bind(rated_id)
        .bind(score)
        .bind(comment)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("ratings_listing_id_rater_id_key")
                    || db_err.constraint() == Some("ratings_listing_id_rater_id")
                    || db_err.code().as_deref() == Some("23505")
                {
                    return RatingError::AlreadyRated;
                }
            }
            tracing::error!("Database error in insert_rating: {}", e);
            RatingError::DatabaseError(e.to_string())
        })?;

        Ok(Rating::from(rating))
    }

    async fn find_by_user_id(
        &self,
        user_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Rating>, RatingError> {
        let rows = sqlx::query_as::<_, RatingRow>(
            r#"SELECT id, listing_id, rater_id, rated_id, score, comment, created_at
               FROM ratings
               WHERE rated_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in find_by_user_id: {}", e);
            RatingError::DatabaseError(e.to_string())
        })?;

        Ok(rows.into_iter().map(Rating::from).collect())
    }

    async fn calculate_average(&self, user_id: Uuid) -> Result<(f64, i64), RatingError> {
        let row: (Option<f64>, Option<i64>) = sqlx::query_as(
            r#"SELECT AVG(score::float8)::float8, COUNT(*)::int8
               FROM ratings
               WHERE rated_id = $1"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in calculate_average: {}", e);
            RatingError::DatabaseError(e.to_string())
        })?;

        Ok((row.0.unwrap_or(0.0), row.1.unwrap_or(0)))
    }

    async fn count_by_user_id(&self, user_id: Uuid) -> Result<i64, RatingError> {
        let count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)::int8 FROM ratings WHERE rated_id = $1"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in count_by_user_id: {}", e);
            RatingError::DatabaseError(e.to_string())
        })?;

        Ok(count.0)
    }

    async fn exists(&self, listing_id: Uuid, rater_id: Uuid) -> Result<bool, RatingError> {
        let row: (bool,) = sqlx::query_as(
            r#"SELECT EXISTS(SELECT 1 FROM ratings WHERE listing_id = $1 AND rater_id = $2)"#,
        )
        .bind(listing_id)
        .bind(rater_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in exists: {}", e);
            RatingError::DatabaseError(e.to_string())
        })?;

        Ok(row.0)
    }
}
