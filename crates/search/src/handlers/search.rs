use axum::extract::{Query, State};
use axum::Json;
use common::errors::AppError;
use sqlx::PgPool;

use crate::adapters::meilisearch_adapter::MeiliSearchAdapter;
use crate::dtos::{SearchQueryDto, SearchResponseDto};
use crate::errors::SearchError;
use crate::usecases::search_usecase;

/// GET /search?q=...&category=...&minPrice=...&maxPrice=...&lat=...&lng=...&radiusKm=...&sort=...&page=...&perPage=...
///
/// Search listings with full-text, filters, and optional geo-radius.
/// Returns results from MeiliSearch or SQL ILIKE fallback.
///
/// This endpoint is **public** (no authentication required).
pub async fn handle_search(
    State(engine): State<Option<MeiliSearchAdapter>>,
    State(pool): State<PgPool>,
    Query(params): Query<SearchQueryDto>,
) -> Result<Json<SearchResponseDto>, AppError> {
    // Validate query parameters
    let validated = params.validate().map_err(|msg| {
        let search_err = SearchError::InvalidParams(msg);
        AppError::from(search_err)
    })?;

    // Get the engine reference (if available) as a trait object
    let engine_ref: Option<&dyn crate::ports::SearchEngine> =
        engine.as_ref().map(|e| e as &dyn crate::ports::SearchEngine);

    let (result, _engine_name) = search_usecase::execute(engine_ref, &pool, validated).await?;

    Ok(Json(result))
}
