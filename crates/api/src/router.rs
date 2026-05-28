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
    // Import routers from each crate

    let ratings_router = ratings::router::ratings_router::<AppState>();
    let favorites_router = favorites::router::favorites_router::<AppState>();
    let geo_router = geo::router::geo_router::<AppState>();

    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Mount each module's router
        .merge(users_router)
        // Mount chat router under /chat prefix
        .merge(chat_router)
        .merge(ratings_router)
        .merge(favorites_router)
        .merge(geo_router)
        // Global middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
