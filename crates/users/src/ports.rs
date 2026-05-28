use async_trait::async_trait;
use uuid::Uuid;

use crate::errors::UserError;
use crate::models::User;

/// UserRepositoryPort defines the interface for user persistence.
/// Following the Dependency Inversion Principle, usecases depend on this
/// trait rather than on concrete infrastructure implementations.
#[async_trait]
pub trait UserRepositoryPort: Send + Sync {
    /// Find a user by their email address.
    /// Returns `Ok(None)` if no user with that email exists.
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserError>;

    /// Find a user by their UUID identifier.
    /// Returns `Ok(None)` if no user with that ID exists.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserError>;

    /// Insert a new user into the database.
    /// Returns `Err(UserError::EmailAlreadyExists)` if the email is already taken.
    async fn insert(
        &self,
        email: &str,
        password_hash: &str,
        display_name: &str,
    ) -> Result<User, UserError>;

    /// Update the `last_login_at` timestamp for a user.
    async fn update_last_login(&self, id: Uuid) -> Result<(), UserError>;
}
