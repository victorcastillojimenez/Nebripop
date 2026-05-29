use axum::routing::get;
use axum::Router;
use sqlx::PgPool;

use crate::adapters::meilisearch_adapter::MeiliSearchAdapter;
use crate::handlers::search;

/// Build the search router with `GET /search`.
///
/// `S` is the outer application state type. Requirements:
/// - `Option<MeiliSearchAdapter>: FromRef<S>` — the search engine (or None = SQL fallback only)
/// - `PgPool: FromRef<S>` — database pool for SQL fallback
pub fn search_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Option<MeiliSearchAdapter>: axum::extract::FromRef<S>,
    PgPool: axum::extract::FromRef<S>,
{
    Router::new().route("/search", get(search::handle_search))
}
