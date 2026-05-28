use sqlx::PgPool;

use crate::adapters::sql_fallback::SqlFallbackAdapter;
use crate::dtos::{SearchQueryDto, SearchResponseDto, SearchResultDto};
use crate::errors::SearchError;
use crate::models::SearchFilters;
use crate::ports::SearchEngine;

/// Execute a search using either MeiliSearch or the SQL ILIKE fallback.
///
/// If `engine` is `Some`, it first tries MeiliSearch.
/// On failure (or if `engine` is `None`), it falls back to SQL ILIKE.
///
/// Returns the search results along with a string indicating which engine
/// served the request (`"meilisearch"` or `"sql_fallback"`).
pub async fn execute(
    engine: Option<&dyn SearchEngine>,
    pool: &PgPool,
    query: SearchQueryDto,
) -> Result<(SearchResponseDto, &'static str), SearchError> {
    let filters = SearchFilters::from(query);

    // Try MeiliSearch first
    if let Some(engine) = engine {
        match engine.search(&filters).await {
            Ok((results, total)) => {
                let dto = build_response(results, total, &filters, "meilisearch");
                return Ok((dto, "meilisearch"));
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "MeiliSearch search failed, falling back to SQL ILIKE"
                );
            }
        }
    } else {
        tracing::debug!("MeiliSearch not configured, using SQL ILIKE fallback");
    }

    // Fallback to SQL ILIKE
    let fallback = SqlFallbackAdapter::new(pool.clone());
    let (results, total) = fallback.search(&filters).await?;
    let dto = build_response(results, total, &filters, "sql_fallback");

    Ok((dto, "sql_fallback"))
}

/// Build a `SearchResponseDto` from raw results.
fn build_response(
    results: Vec<crate::models::SearchResult>,
    total: i64,
    filters: &SearchFilters,
    engine: &str,
) -> SearchResponseDto {
    let items: Vec<SearchResultDto> = results
        .into_iter()
        .map(|r| SearchResultDto {
            id: r.id,
            title: r.title,
            price: r.price,
            currency: r.currency,
            category: r.category,
            condition: r.condition,
            city: r.city,
            image_url: r.image_url,
            distance_km: r.distance_km,
            created_at: r.created_at,
        })
        .collect();

    let page = filters.page;
    let per_page = filters.limit();

    SearchResponseDto::new(items, total, page, per_page, engine)
}
