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
    // Import sub-routers
    let users_router = users::router::users_router::<AppState>();
    let chat_router = chat::router::chat_router::<AppState>();

    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Mount users router directly (it already has /auth and /users paths)
        .merge(users_router)
        // Mount chat router under /chat prefix
        .merge(chat_router)
        // Global middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
