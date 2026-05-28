use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::SearchError;
use crate::models::{ListingDoc, SearchFilters, SearchResult};
use crate::ports::SearchEngine;

/// Adapter that implements SearchEngine using PostgreSQL ILIKE queries.
/// Used as a fallback when MeiliSearch is not available.
#[derive(Debug, Clone)]
pub struct SqlFallbackAdapter {
    pool: PgPool,
}

impl SqlFallbackAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Private row type for SQL fallback queries.
/// Price is cast to FLOAT8 in SQL for direct f64 mapping.
#[derive(Debug, sqlx::FromRow)]
struct SqlResultRow {
    id: Uuid,
    title: String,
    price: f64,
    currency: String,
    category: String,
    condition: String,
    city: String,
    image_url: Option<String>,
    created_at: DateTime<Utc>,
}

#[async_trait]
impl SearchEngine for SqlFallbackAdapter {
    async fn search(
        &self,
        filters: &SearchFilters,
    ) -> Result<(Vec<SearchResult>, i64), SearchError> {
        let per_page = filters.limit();
        let offset = filters.offset();

        // ── Build WHERE clause using $1, $2, ... positional params ──
        // The order of conditions must match the order of bind() calls.
        // Condition order: q (ILIlKE), category, min_price, max_price.
        let mut conditions: Vec<String> = vec!["l.status = 'active'".to_string()];
        let mut param_idx = 0u32;

        // Track which filters are active to know how many binds to issue
        let has_q = filters.q.is_some();
        let has_category = filters.category.is_some();
        let has_min_price = filters.min_price.is_some();
        let has_max_price = filters.max_price.is_some();

        if has_q {
            param_idx += 1;
            conditions.push(format!(
                "(l.title ILIKE ${idx} OR l.description ILIKE ${idx})",
                idx = param_idx
            ));
        }
        if has_category {
            param_idx += 1;
            conditions.push(format!("l.category = ${}", param_idx));
        }
        if has_min_price {
            param_idx += 1;
            conditions.push(format!("l.price >= ${}::numeric", param_idx));
        }
        if has_max_price {
            param_idx += 1;
            conditions.push(format!("l.price <= ${}::numeric", param_idx));
        }

        let where_clause = conditions.join(" AND ");
        let limit_param = param_idx + 1;
        let offset_param = param_idx + 2;

        // ── Build COUNT query ──
        let count_sql = format!(
            "SELECT COUNT(*)::int8 FROM listings l WHERE {}",
            where_clause
        );

        // ── Build SELECT query ──
        let select_sql = format!(
            r#"SELECT l.id, l.title, l.price::float8 AS "price", l.currency, l.category,
                      l.condition, l.city, l.created_at,
                      (SELECT li.image_url FROM listing_images li
                       WHERE li.listing_id = l.id
                       ORDER BY li.position ASC LIMIT 1) AS "image_url"
               FROM listings l
               WHERE {}
               ORDER BY l.created_at DESC
               LIMIT ${} OFFSET ${}"#,
            where_clause, limit_param, offset_param,
        );

        // ── Execute COUNT ──
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
        if has_q {
            let pattern = format!("%{}%", filters.q.as_deref().unwrap_or(""));
            count_query = count_query.bind(pattern);
        }
        if has_category {
            count_query = count_query.bind(filters.category.as_deref().unwrap_or(""));
        }
        if has_min_price {
            count_query = count_query.bind(filters.min_price.unwrap_or(0.0));
        }
        if has_max_price {
            count_query = count_query.bind(filters.max_price.unwrap_or(0.0));
        }

        let total = count_query.fetch_one(&self.pool).await.map_err(|e| {
            tracing::error!("SQL fallback count query failed: {}", e);
            SearchError::DatabaseError(format!("Error al contar resultados: {e}"))
        })?;

        // ── Execute SELECT ──
        let mut select_query = sqlx::query_as::<_, SqlResultRow>(&select_sql);
        if has_q {
            let pattern = format!("%{}%", filters.q.as_deref().unwrap_or(""));
            select_query = select_query.bind(pattern);
        }
        if has_category {
            select_query = select_query.bind(filters.category.as_deref().unwrap_or(""));
        }
        if has_min_price {
            select_query = select_query.bind(filters.min_price.unwrap_or(0.0));
        }
        if has_max_price {
            select_query = select_query.bind(filters.max_price.unwrap_or(0.0));
        }
        select_query = select_query.bind(per_page).bind(offset);

        let rows: Vec<SqlResultRow> = select_query.fetch_all(&self.pool).await.map_err(|e| {
            tracing::error!("SQL fallback select query failed: {}", e);
            SearchError::DatabaseError(format!("Error al buscar anuncios: {e}"))
        })?;

        // ── Map rows to SearchResult ──
        let items: Vec<SearchResult> = rows
            .into_iter()
            .map(|row| SearchResult {
                id: row.id,
                title: row.title,
                price: row.price,
                currency: row.currency,
                category: row.category,
                condition: row.condition,
                city: row.city,
                image_url: row.image_url,
                distance_km: None, // SQL fallback doesn't compute distance
                created_at: row.created_at.timestamp(),
            })
            .collect();

        Ok((items, total.0))
    }

    /// SQL fallback does NOT support index operations (read-only).
    async fn index_listing(&self, _doc: &ListingDoc) -> Result<(), SearchError> {
        tracing::warn!("SQL fallback does not support index_listing — skipped");
        Ok(())
    }

    /// SQL fallback does NOT support index operations (read-only).
    async fn remove_listing(&self, _id: Uuid) -> Result<(), SearchError> {
        tracing::warn!("SQL fallback does not support remove_listing — skipped");
        Ok(())
    }
}
