use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use crate::web;

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
    let ratings_router = ratings::router::ratings_router::<AppState>();
    let favorites_router = favorites::router::favorites_router::<AppState>();
    let geo_router = geo::router::geo_router::<AppState>();
    let listings_router = listings::router::listings_router::<AppState>();
    let payments_router = payments::router::payments_router::<AppState>();
    let search_router = search::router::search_router::<AppState>();

    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Mount each module's router
        .merge(users_router)
        .merge(chat_router)
        .merge(ratings_router)
        .merge(favorites_router)
        .merge(geo_router)
        .merge(listings_router)
        .merge(payments_router)
        .merge(search_router)
        // Web Handlers
        .route("/", get(web::home::home_handler))
        .route("/listings", get(web::listings::listings_handler))
        .route("/listings/:id", get(web::listing_detail::listing_detail_handler))
        .route("/listings/new", get(web::listing_create::listing_create_handler))
        .route("/search", get(web::search::search_handler))
        .route("/login", get(web::auth::login_handler))
        .route("/register", get(web::auth::register_handler))
        .route("/users/:id", get(web::profile::profile_handler))
        .route("/chat", get(web::chat_web::chat_list_handler))
        .route("/chat/:id", get(web::chat_web::conversation_handler))
        .route("/pay", get(web::checkout::checkout_handler))
        .route("/pay/success", get(web::checkout::payment_success_handler))
        .route("/pay/error", get(web::checkout::payment_error_handler))
        // Global middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
