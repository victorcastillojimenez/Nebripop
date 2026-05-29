use axum::routing::{get, post};
use axum::Router;

use crate::adapters::listing_adapter::PaymentsListingAdapter;
use crate::adapters::payment_repository::PostgresPaymentRepository;
use crate::adapters::stripe_adapter::StripeAdapter;
use crate::handlers::create_intent::handle_create_intent;
use crate::handlers::get_status::handle_get_status;
use crate::handlers::webhook::handle_webhook;

/// Build the payments sub-router.
///
/// Routes:
/// - POST /payments/intent  (requires JWT auth)
/// - POST /payments/webhook (Stripe signature auth, NO JWT)
/// - GET  /payments/:id/status (requires JWT auth)
///
/// Uses concrete AppState via FromRef constraints.
pub fn payments_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    PostgresPaymentRepository: axum::extract::FromRef<S>,
    StripeAdapter: axum::extract::FromRef<S>,
    PaymentsListingAdapter: axum::extract::FromRef<S>,
    String: axum::extract::FromRef<S>,
{
    Router::new()
        .route("/payments/intent", post(handle_create_intent))
        .route("/payments/webhook", post(handle_webhook))
        .route("/payments/{id}/status", get(handle_get_status))
}
