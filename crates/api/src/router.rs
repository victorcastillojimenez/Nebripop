use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::app_state::AppState;
use crate::web::home::home_handler;
use crate::web::listings::listings_handler;
use crate::web::listing_detail::listing_detail_handler;
use crate::web::listing_create::{listing_create_handler, listing_create_post_handler};
use crate::web::search::search_handler;
use crate::web::auth::{login_handler, login_post_handler, register_handler, register_post_handler, logout_handler};
use crate::web::profile::{profile_handler, my_profile_handler};
use crate::web::chat_web::{chat_list_handler, conversation_handler};
use crate::web::checkout::{checkout_handler, payment_success_handler, payment_error_handler};

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
        // API JSON Routes under /api
        .nest(
            "/api",
            Router::new()
                .merge(users_router)
                .merge(listings_router)
                .merge(search_router)
                .merge(ratings_router)
                .merge(favorites_router)
                .merge(geo_router)
                .merge(chat_router)
                .merge(payments_router),
        )
        // HTML Pages mounted at root
        .route("/", get(home_handler))
        .route("/listings", get(listings_handler))
        .route("/listings/:id", get(listing_detail_handler))
        .route("/listings/new", get(listing_create_handler))
        .route("/listings/create", get(listing_create_handler).post(listing_create_post_handler))
        .route("/search", get(search_handler))
        .route("/login", get(login_handler).post(login_post_handler))
        .route("/register", get(register_handler).post(register_post_handler))
        .route("/logout", get(logout_handler))
        .route("/users/me", get(my_profile_handler))
        .route("/users/:id", get(profile_handler))
        .route("/me", get(my_profile_handler))
        .route("/chat", get(chat_list_handler))
        .route("/chat/:id", get(conversation_handler))
        .route("/payments/checkout/:id", get(checkout_handler))
        .route("/payments/success", get(payment_success_handler))
        .route("/payments/error", get(payment_error_handler))
        .route("/health", get(health_check))
        // Global middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
