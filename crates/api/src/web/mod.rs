pub mod home;
pub mod listings;
pub mod listing_detail;
pub mod listing_create;
pub mod search;
pub mod auth;
pub mod profile;
pub mod chat_web;
pub mod checkout;
pub mod filters;

use axum::{routing::get, Router};
use crate::app_state::AppState;

pub fn web_router() -> Router<AppState> {
    Router::new()
        .route("/", get(home::home_handler))
        .route("/listings", get(listings::listings_handler))
        .route("/listings/:id", get(listing_detail::listing_detail_handler))
        .route("/listings/new", get(listing_create::listing_create_handler))
        .route("/search", get(search::search_handler))
        .route("/login", get(auth::login_handler).post(auth::login_post_handler))
        .route("/register", get(auth::register_handler).post(auth::register_post_handler))
        .route("/logout", get(auth::logout_handler))
        .route("/users/:id", get(profile::profile_handler))
        .route("/chat", get(chat_web::chat_list_handler))
        .route("/chat/:id", get(chat_web::conversation_handler))
        .route("/payments/checkout/:id", get(checkout::checkout_handler))
        .route("/payments/success", get(checkout::payment_success_handler))
        .route("/payments/error", get(checkout::payment_error_handler))
}

