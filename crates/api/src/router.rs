use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::app_state::AppState;

/// Health check handler
pub async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": "1.0.0"
    }))
}

/// Build the main application router with all sub-routers mounted
pub fn build_router() -> Router<AppState> {
    // Import users router
    let users_router = users::router::users_router::<AppState>();

    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Mount users router directly (it already has /auth and /users paths)
        .merge(users_router)
        // Global middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
