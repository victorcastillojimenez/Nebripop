use axum::routing::{get, post};
use axum::Router;

use crate::handlers;

/// State type that must be provided when mounting this router
/// The state must contain:
/// - UserRepository (via State extractor)
/// - String jwt_secret (via State extractor)
pub fn users_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    crate::adapters::user_repository::UserRepository: axum::extract::FromRef<S>,
    String: axum::extract::FromRef<S>,
{
    Router::new()
        // Auth routes
        .route("/auth/register", post(handlers::register::register_handler))
        .route("/auth/login", post(handlers::login::login_handler))
        .route("/auth/refresh", post(handlers::refresh::refresh_handler))
        .route("/auth/logout", post(handlers::logout::logout_handler))
        // User profile routes
        .route("/users/:id", get(handlers::get_profile::get_profile_handler))
}
