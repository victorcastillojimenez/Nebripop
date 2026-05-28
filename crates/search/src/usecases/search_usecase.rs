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

    if let Some(engine_instance) = engine {
        if let Some(res) = try_meilisearch(engine_instance, &filters).await {
            return Ok(res);
        }
    } else {
        tracing::debug!("MeiliSearch not configured, using SQL ILIKE fallback");
    }

    let fallback = SqlFallbackAdapter::new(pool.clone());
    let (results, total) = fallback.search(&filters).await?;
    let dto = build_response(results, total, &filters, "sql_fallback");

    Ok((dto, "sql_fallback"))
}

async fn try_meilisearch(
    engine: &dyn SearchEngine,
    filters: &SearchFilters,
) -> Option<(SearchResponseDto, &'static str)> {
    match engine.search(filters).await {
        Ok((results, total)) => {
            let dto = build_response(results, total, filters, "meilisearch");
            Some((dto, "meilisearch"))
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "MeiliSearch search failed, falling back to SQL ILIKE"
            );
            None
        }
    }
}

/// Build a `SearchResponseDto` from raw results.
fn build_response(
    results: Vec<crate::models::SearchResult>,
    total: i64,
    filters: &SearchFilters,
    engine: &str,
) -> SearchResponseDto {
    let items = results.into_iter().map(SearchResultDto::from).collect();
    SearchResponseDto::new(items, total, filters.page, filters.limit(), engine)
}
