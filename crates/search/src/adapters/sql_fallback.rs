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

#[async_trait]
impl SearchEngine for SqlFallbackAdapter {
    async fn search(
        &self,
        filters: &SearchFilters,
    ) -> Result<(Vec<SearchResult>, i64), SearchError> {
        let per_page = filters.limit();
        let offset = filters.offset();
        let has_geo = filters.lat.is_some() && filters.lng.is_some();

        // ── Reserve parameter indices ──
        // Bind order: q, category, min_price, max_price, radius_m, lat, lng, limit, offset
        let mut pt = ParamTracker::new();

        let q_idx = if filters.q.is_some() { Some(pt.reserve()) } else { None };
        let cat_idx = if filters.category.is_some() { Some(pt.reserve()) } else { None };
        let min_p_idx = if filters.min_price.is_some() { Some(pt.reserve()) } else { None };
        let max_p_idx = if filters.max_price.is_some() { Some(pt.reserve()) } else { None };

        let (radius_idx, lat_idx, lng_idx) = if has_geo {
            (
                Some(pt.reserve()),
                Some(pt.reserve()),
                Some(pt.reserve()),
            )
        } else {
            (None, None, None)
        };

        let limit_idx = pt.reserve();
        let offset_idx = pt.reserve();

        // ── Build WHERE conditions ──
        let mut conditions: Vec<String> = vec!["l.status = 'active'".to_string()];

        if let Some(idx) = q_idx {
            conditions.push(format!(
                "(l.title ILIKE ${idx} OR l.description ILIKE ${idx})",
                idx = idx
            ));
        }
        if let Some(idx) = cat_idx {
            conditions.push(format!("l.category = ${idx}", idx = idx));
        }
        if let Some(idx) = min_p_idx {
            conditions.push(format!("l.price >= ${idx}::numeric", idx = idx));
        }
        if let Some(idx) = max_p_idx {
            conditions.push(format!("l.price <= ${idx}::numeric", idx = idx));
        }
        if let (Some(r_idx), Some(la_idx), Some(lo_idx)) = (radius_idx, lat_idx, lng_idx) {
            conditions.push(format!(
                "ST_DWithin(l.location, ST_SetSRID(ST_MakePoint(${lng}, ${lat}), 4326), ${radius})",
                lng = lo_idx,
                lat = la_idx,
                radius = r_idx,
            ));
        }

        let where_clause = conditions.join(" AND ");

        // ── ORDER BY ──
        let order_clause = if has_geo {
            "ORDER BY distance_km ASC".to_string()
        } else {
            "ORDER BY l.created_at DESC".to_string()
        };

        // ── Distance expression for SELECT ──
        let distance_expr = if let (Some(la_idx), Some(lo_idx)) = (lat_idx, lng_idx) {
            format!(
                "ST_Distance(l.location, ST_SetSRID(ST_MakePoint(${lng}, ${lat}), 4326)::geography) / 1000.0 AS distance_km",
                lng = lo_idx,
                lat = la_idx,
            )
        } else {
            "NULL::float8 AS distance_km".to_string()
        };

        // ── Build SELECT query ──
        let select_sql = format!(
            r#"SELECT l.id, l.title, l.price::float8 AS "price", l.currency, l.category,
                      l.condition, l.city, l.created_at,
                      (SELECT li.image_url FROM listing_images li
                       WHERE li.listing_id = l.id
                       ORDER BY li.position ASC LIMIT 1) AS "image_url",
                      {distance}
               FROM listings l
               WHERE {where}
               {order}
               LIMIT ${limit} OFFSET ${offset}"#,
            distance = distance_expr,
            where = where_clause,
            order = order_clause,
            limit = limit_idx,
            offset = offset_idx,
        );

        // ── Build COUNT query (same WHERE, no geo columns needed) ──
        let count_sql = format!(
            "SELECT COUNT(*)::int8 FROM listings l WHERE {where}",
            where = where_clause,
        );

        // ── Execute COUNT ──
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(ref q) = filters.q {
            count_query = count_query.bind(format!("%{}%", q));
        }
        if let Some(ref cat) = filters.category {
            count_query = count_query.bind(cat);
        }
        if let Some(min) = filters.min_price {
            count_query = count_query.bind(min);
        }
        if let Some(max) = filters.max_price {
            count_query = count_query.bind(max);
        }
        if let (Some(_), Some(lat), Some(lng)) = (radius_idx, filters.lat, filters.lng) {
            let radius_m = filters.radius_km.unwrap_or(50.0).max(1.0) * 1000.0;
            count_query = count_query.bind(radius_m);
            count_query = count_query.bind(lat);
            count_query = count_query.bind(lng);
        }

        let total = count_query.fetch_one(&self.pool).await.map_err(|e| {
            tracing::error!("SQL fallback count query failed: {}", e);
            SearchError::DatabaseError(format!("Error al contar resultados: {e}"))
        })?;

        // ── Execute SELECT ──
        let mut select_query = sqlx::query_as::<_, SqlResultRow>(&select_sql);
        if let Some(ref q) = filters.q {
            select_query = select_query.bind(format!("%{}%", q));
        }
        if let Some(ref cat) = filters.category {
            select_query = select_query.bind(cat);
        }
        if let Some(min) = filters.min_price {
            select_query = select_query.bind(min);
        }
        if let Some(max) = filters.max_price {
            select_query = select_query.bind(max);
        }
        if let (Some(_), Some(lat), Some(lng)) = (radius_idx, filters.lat, filters.lng) {
            let radius_m = filters.radius_km.unwrap_or(50.0).max(1.0) * 1000.0;
            select_query = select_query.bind(radius_m);
            select_query = select_query.bind(lat);
            select_query = select_query.bind(lng);
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
                distance_km: row.distance_km,
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
