use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::UserError;
use crate::models::User;
use crate::ports::UserRepositoryPort;

#[derive(Debug, Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepositoryPort for UserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, display_name, avatar_url, phone, role, rating_avg, total_ratings, last_login_at, created_at, updated_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, display_name, avatar_url, phone, role, rating_avg, total_ratings, last_login_at, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn insert(
        &self,
        email: &str,
        password_hash: &str,
        display_name: &str,
    ) -> Result<User, UserError> {
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, $3) RETURNING id, email, password_hash, display_name, avatar_url, phone, role, rating_avg, total_ratings, last_login_at, created_at, updated_at",
        )
        .bind(email)
        .bind(password_hash)
        .bind(display_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("users_email_key") {
                    return UserError::EmailAlreadyExists;
                }
            }
            UserError::DatabaseError(e)
        })?;

        Ok(user)
    }

    async fn update_last_login(&self, id: Uuid) -> Result<(), UserError> {
        sqlx::query("UPDATE users SET last_login_at = $1, updated_at = $1 WHERE id = $2")
            .bind(Utc::now())
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
