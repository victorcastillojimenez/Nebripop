use async_trait::async_trait;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::indexes::Index;
use meilisearch_sdk::search::SearchResults;
use serde_json::Value;
use uuid::Uuid;

use crate::errors::SearchError;
use crate::models::{ListingDoc, SearchFilters, SearchResult};
use crate::ports::SearchEngine;

const LISTINGS_INDEX: &str = "listings_index";

#[derive(Debug, Clone)]
pub struct MeiliSearchAdapter {
    client: Client,
}

impl MeiliSearchAdapter {
    /// Create a new MeiliSearch adapter.
    ///
    /// # Errors
    ///
    /// Returns `SearchError::MeiliSearchError` if the client cannot connect.
    pub fn new(url: &str, api_key: Option<&str>) -> Result<Self, SearchError> {
        let client = match api_key {
            Some(key) => Client::new(url, Some(key)),
            None => Client::new(url, None::<&str>),
        }
        .map_err(|e| SearchError::MeiliSearchError(format!("Failed to create Meilisearch client: {e}")))?;

        Ok(Self { client })
    }

    /// Get the existing index or create it if it does not exist (idempotent).
    async fn get_or_create_index(&self) -> Result<Index, SearchError> {
        // List existing indexes to check if ours already exists
        let existing = self
            .client
            .list_all_indexes()
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to list indexes: {e}")))?;

        if existing.results.iter().any(|i| i.uid == LISTINGS_INDEX) {
            return Ok(self.client.index(LISTINGS_INDEX));
        }

        // Index does not exist — create it
        let task = self
            .client
            .create_index(LISTINGS_INDEX, Some("id"))
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to create index: {e}")))?;

        self.client
            .wait_for_task(
                task,
                Some(std::time::Duration::from_millis(500)),
                Some(std::time::Duration::from_secs(5)),
            )
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed waiting for index creation: {e}")))?;

        Ok(self.client.index(LISTINGS_INDEX))
    }

    /// Ensure the index exists with the correct settings (idempotent).
    pub async fn setup_index(&self) -> Result<(), SearchError> {
        let index = self.get_or_create_index().await?;

        index
            .set_searchable_attributes(["title", "description", "city"])
            .await?;

        index
            .set_filterable_attributes(["category", "price", "status", "_geo"])
            .await?;

        index
            .set_sortable_attributes(["price", "created_at"])
            .await?;

        tracing::info!("MeiliSearch index '{LISTINGS_INDEX}' configured successfully");

        Ok(())
    }

    fn build_filter(filters: &SearchFilters) -> String {
        let mut parts: Vec<String> = Vec::new();

        parts.push("status = 'active'".to_string());

        if let Some(ref cat) = filters.category {
            let escaped = cat.replace('\'', "\\'");
            parts.push(format!("category = '{}'", escaped));
        }

        if let Some(min) = filters.min_price {
            parts.push(format!("price >= {min}"));
        }
        if let Some(max) = filters.max_price {
            parts.push(format!("price <= {max}"));
        }

        if let (Some(lat), Some(lng)) = (filters.lat, filters.lng) {
            let radius_m = filters.radius_km.unwrap_or(50.0).max(1.0) * 1000.0;
            parts.push(format!("_geoRadius({lat}, {lng}, {radius_m})"));
        }

        parts.join(" AND ")
    }

    fn build_sort(sort: Option<&str>) -> Option<Vec<&'static str>> {
        match sort {
            Some("price_asc") => Some(vec!["price:asc"]),
            Some("price_desc") => Some(vec!["price:desc"]),
            Some("date_desc") => Some(vec!["created_at:desc"]),
            _ => None,
        }
    }
}

#[async_trait]
impl SearchEngine for MeiliSearchAdapter {
    async fn search(
        &self,
        filters: &SearchFilters,
    ) -> Result<(Vec<SearchResult>, i64), SearchError> {
        let index = self.client.index(LISTINGS_INDEX);
        let limit = filters.limit() as usize;
        let offset = filters.offset() as usize;

        // Build filter, query, and sort strings with sufficient lifetimes
        let query_owned = filters.q.as_deref().unwrap_or("").to_string();
        let filter_owned = Self::build_filter(filters);
        let sort_vec: Vec<&'static str> = Self::build_sort(filters.sort.as_deref()).unwrap_or_default();

        // ── Build the MeiliSearch query ──
        let mut search = index.search();
        search.with_query(&query_owned);

        if !filter_owned.is_empty() {
            search.with_filter(&filter_owned);
        }

        if !sort_vec.is_empty() {
            search.with_sort(&sort_vec);
        }

        search.with_limit(limit);
        search.with_offset(offset);

        let results: SearchResults<Value> = search
            .execute()
            .await
            .map_err(|e| SearchError::MeiliSearchError(format!("Search query failed: {e}")))?;

        let items: Vec<SearchResult> = results
            .hits
            .iter()
            .filter_map(|hit| {
                let doc = &hit.result;
                let id = Uuid::parse_str(doc.get("id")?.as_str()?).ok()?;
                let title = doc.get("title")?.as_str()?.to_string();
                let price = doc.get("price")?.as_f64()?;
                let currency = doc
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .unwrap_or("eur")
                    .to_string();
                let category = doc.get("category")?.as_str()?.to_string();
                let condition = doc.get("condition")?.as_str()?.to_string();
                let city = doc.get("city")?.as_str()?.to_string();
                let image_url = doc
                    .get("image_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let created_at = doc.get("created_at")?.as_i64()?;

                let distance_km = hit
                    .formatted_result
                    .as_ref()
                    .and_then(|f| f.get("_geo"))
                    .and_then(|geo| geo.get("distance"))
                    .and_then(|d| d.as_f64())
                    .map(|d| d / 1000.0);

                Some(SearchResult {
                    id,
                    title,
                    price,
                    currency,
                    category,
                    condition,
                    city,
                    image_url,
                    distance_km,
                    created_at,
                })
            })
            .collect();

        let total = results.estimated_total_hits.unwrap_or(items.len()) as i64;

        Ok((items, total))
    }

    async fn index_listing(&self, doc: &ListingDoc) -> Result<(), SearchError> {
        let index = self.client.index(LISTINGS_INDEX);

        index
            .add_documents(&[doc.clone()], Some("id"))
            .await
            .map_err(|e| SearchError::MeiliSearchError(format!("Failed to index listing: {e}")))?;

        Ok(())
    }

    async fn remove_listing(&self, id: Uuid) -> Result<(), SearchError> {
        let index = self.client.index(LISTINGS_INDEX);

        index
            .delete_document(id.to_string())
            .await
            .map_err(|e| SearchError::MeiliSearchError(format!("Failed to remove listing: {e}")))?;

        Ok(())
    }
}