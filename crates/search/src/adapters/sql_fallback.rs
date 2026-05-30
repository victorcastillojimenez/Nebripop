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
/// `distance_km` is computed via PostGIS ST_Distance when lat/lng filters are present.
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
    /// Distance in km (NULL when geo not used).
    distance_km: Option<f64>,
}

/// Internal helper to track parameter positions for dynamic SQL binding.
/// All indices are 1-based (matching PostgreSQL `$N` notation).
struct ParamTracker {
    next: u32,
}

impl ParamTracker {
    fn new() -> Self {
        Self { next: 1 }
    }

    /// Reserve the next parameter index and return it.
    fn reserve(&mut self) -> u32 {
        let idx = self.next;
        self.next += 1;
        idx
    }

    fn current(&self) -> u32 {
        self.next
    }
}

fn build_where_clause(
    filters: &SearchFilters,
    has_geo: bool,
    pt: &mut ParamTracker,
) -> (String, Option<u32>, Option<u32>, Option<u32>) {
    let mut conditions = vec!["l.status = 'active'".to_string()];
    if filters.query.is_some() {
        let idx = pt.reserve();
        conditions.push(format!("(l.title ILIKE ${idx} OR l.description ILIKE ${idx})"));
    }
    if filters.category.is_some() { conditions.push(format!("l.category = ${}", pt.reserve())); }
    if filters.min_price.is_some() { conditions.push(format!("l.price >= ${}::numeric", pt.reserve())); }
    if filters.max_price.is_some() { conditions.push(format!("l.price <= ${}::numeric", pt.reserve())); }
    if let Some(ref cond_values) = filters.condition {
        if cond_values.len() == 1 {
            let idx = pt.reserve();
            conditions.push(format!("l.condition = ${idx}"));
        } else if cond_values.len() > 1 {
            let params: Vec<String> = cond_values.iter().map(|_| format!("${}", pt.reserve())).collect();
            conditions.push(format!("l.condition IN ({})", params.join(", ")));
        }
    }
    let (r_idx, la_idx, lo_idx) = if has_geo {
        let r = pt.reserve();
        let (la, lo) = (pt.reserve(), pt.reserve());
        conditions.push(format!("ST_DWithin(l.location, ST_SetSRID(ST_MakePoint(${lo}, ${la}), 4326), ${r})"));
        (Some(r), Some(la), Some(lo))
    } else { (None, None, None) };
    (conditions.join(" AND "), r_idx, la_idx, lo_idx)
}

fn build_select_sql(where_clause: &str, has_geo: bool, limit_idx: u32, offset_idx: u32, la_idx: Option<u32>, lo_idx: Option<u32>) -> String {
    let order = if has_geo { "ORDER BY distance_km ASC" } else { "ORDER BY l.created_at DESC" };
    let distance = if let (Some(la), Some(lo)) = (la_idx, lo_idx) {
        format!("ST_Distance(l.location, ST_SetSRID(ST_MakePoint(${lo}, ${la}), 4326)::geography) / 1000.0 AS distance_km")
    } else {
        "NULL::float8 AS distance_km".to_string()
    };
    format!(
        r#"SELECT l.id, l.title, l.price::float8 AS "price", l.currency, l.category,
                  l.condition, l.city, l.created_at,
                  (SELECT li.image_url FROM listing_images li WHERE li.listing_id = l.id ORDER BY li.position ASC LIMIT 1) AS "image_url",
                  {distance}
           FROM listings l WHERE {where_clause} {order} LIMIT ${limit_idx} OFFSET ${offset_idx}"#
    )
}

async fn execute_count(pool: &PgPool, sql: &str, filters: &SearchFilters, has_geo: bool) -> Result<i64, SearchError> {
    let mut query = sqlx::query_as::<_, (i64,)>(sql);
    if let Some(ref q) = filters.query { query = query.bind(format!("%{}%", q)); }
    if let Some(ref cat) = filters.category { query = query.bind(cat); }
    if let Some(min) = filters.min_price { query = query.bind(min); }
    if let Some(max) = filters.max_price { query = query.bind(max); }
    if let Some(ref conditions) = filters.condition {
        for cond in conditions {
            query = query.bind(cond);
        }
    }
    if has_geo {
        if let (Some(lat), Some(lng)) = (filters.latitude, filters.longitude) {
            let radius_m = filters.radius_km.unwrap_or(50.0).max(1.0) * 1000.0;
            query = query.bind(radius_m).bind(lat).bind(lng);
        }
    }
    let res = query.fetch_one(pool).await.map_err(|e| {
        tracing::error!("SQL count failed: {e}");
        SearchError::DatabaseError(format!("Error al contar resultados: {e}"))
    })?;
    Ok(res.0)
}

async fn execute_select(
    pool: &PgPool,
    sql: &str,
    filters: &SearchFilters,
    has_geo: bool,
) -> Result<Vec<SqlResultRow>, SearchError> {
    let mut query = sqlx::query_as::<_, SqlResultRow>(sql);
    if let Some(ref q) = filters.query { query = query.bind(format!("%{}%", q)); }
    if let Some(ref cat) = filters.category { query = query.bind(cat); }
    if let Some(min) = filters.min_price { query = query.bind(min); }
    if let Some(max) = filters.max_price { query = query.bind(max); }
    if let Some(ref conditions) = filters.condition {
        for cond in conditions {
            query = query.bind(cond);
        }
    }
    if has_geo {
        if let (Some(lat), Some(lng)) = (filters.latitude, filters.longitude) {
            let radius_m = filters.radius_km.unwrap_or(50.0).max(1.0) * 1000.0;
            query = query.bind(radius_m).bind(lat).bind(lng);
        }
    }
    query = query.bind(filters.limit()).bind(filters.offset());
    query.fetch_all(pool).await.map_err(|e| {
        tracing::error!("SQL select failed: {e}");
        SearchError::DatabaseError(format!("Error al buscar anuncios: {e}"))
    })
}

fn map_row_to_result(row: SqlResultRow) -> SearchResult {
    SearchResult {
        id: row.id,
        title: row.title,
        price: row.price,
        currency: row.currency,
        category: row.category,
        condition: row.condition,
        city: row.city,
        image_url: row.image_url,
        distance_km: row.distance_km,
        created_at: row.created_at.timestamp(),
    }
}

#[async_trait]
impl SearchEngine for SqlFallbackAdapter {
    async fn search(
        &self,
        filters: &SearchFilters,
    ) -> Result<(Vec<SearchResult>, i64), SearchError> {
        let has_geo = filters.latitude.is_some() && filters.longitude.is_some();
        let mut pt = ParamTracker::new();
        let (where_clause, _, la_idx, lo_idx) = build_where_clause(filters, has_geo, &mut pt);

        let limit_idx = pt.reserve();
        let offset_idx = pt.reserve();
        let select_sql = build_select_sql(&where_clause, has_geo, limit_idx, offset_idx, la_idx, lo_idx);
        let count_sql = format!("SELECT COUNT(*)::int8 FROM listings l WHERE {where_clause}");

        let total = execute_count(&self.pool, &count_sql, filters, has_geo).await?;
        let rows = execute_select(&self.pool, &select_sql, filters, has_geo).await?;
        let items = rows.into_iter().map(map_row_to_result).collect();

        Ok((items, total))
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
