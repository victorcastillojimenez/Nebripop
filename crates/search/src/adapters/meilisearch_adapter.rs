use async_trait::async_trait;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::indexes::Index;
use meilisearch_sdk::search::SearchResults;
use serde_json::Value;
use uuid::Uuid;

use crate::errors::SearchError;
use crate::models::{ListingDoc, SearchFilters, SearchResult};
use crate::ports::SearchEngine;

/// Name of the MeiliSearch index for listings.
const LISTINGS_INDEX: &str = "listings_index";

/// Maximum number of results per page.
const MAX_PER_PAGE: i64 = 100;

/// Adapter that implements SearchEngine using MeiliSearch.
#[derive(Debug, Clone)]
pub struct MeiliSearchAdapter {
    client: Client,
}

impl MeiliSearchAdapter {
    /// Create a new MeiliSearch adapter.
    /// Connects to the given URL with the optional API key.
    pub fn new(url: &str, api_key: Option<&str>) -> Self {
        let client = match api_key {
            Some(key) => Client::new(url, key),
            None => Client::new(url, ""),
        };
        Self { client }
    }

    /// Get a reference to the listings index, creating it if needed.
    async fn get_or_create_index(&self) -> Result<Index, SearchError> {
        // Try to get existing index; if it doesn't exist, create it
        let task = self
            .client
            .create_index(LISTINGS_INDEX, Some("id"))
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to create index: {e}")))?;

        // Wait for the task to complete
        let _ = self
            .client
            .wait_for_task(task.uid, std::time::Duration::from_secs(5))
            .await;

        let index = self
            .client
            .index(LISTINGS_INDEX);

        Ok(index)
    }

    /// Configure the index settings (filterable, sortable, searchable attributes).
    pub async fn setup_index(&self) -> Result<(), SearchError> {
        let index = self.get_or_create_index().await?;

        // Set searchable attributes
        index
            .set_searchable_attributes(["title", "description", "city"])
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to set searchable attributes: {e}")))?;

        // Set filterable attributes
        index
            .set_filterable_attributes(["category", "price", "status", "_geo"])
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to set filterable attributes: {e}")))?;

        // Set sortable attributes
        index
            .set_sortable_attributes(["price", "created_at"])
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to set sortable attributes: {e}")))?;

        tracing::info!("MeiliSearch index '{}' configured successfully", LISTINGS_INDEX);

        Ok(())
    }

    /// Build a MeiliSearch filter string from the given params.
    fn build_filter(filters: &SearchFilters) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Always filter by active listings
        parts.push("status = 'active'".to_string());

        // Category filter
        if let Some(ref cat) = filters.category {
            // Escape single quotes in category value
            let escaped = cat.replace('\'', "\\'");
            parts.push(format!("category = '{}'", escaped));
        }

        // Price range
        if let Some(min) = filters.min_price {
            parts.push(format!("price >= {}", min));
        }
        if let Some(max) = filters.max_price {
            parts.push(format!("price <= {}", max));
        }

        // Geo-radius filter (if lat/lng provided)
        if let (Some(lat), Some(lng)) = (filters.lat, filters.lng) {
            let radius_m = filters
                .radius_km
                .unwrap_or(50.0)
                .max(1.0) // minimum 1 km
                * 1000.0; // convert km to meters
            parts.push(format!("_geoRadius({}, {}, {})", lat, lng, radius_m));
        }

        parts.join(" AND ")
    }

    /// Build a MeiliSearch sort string from the given sort param.
    fn build_sort(sort: Option<&str>) -> Option<Vec<String>> {
        match sort {
            Some("price_asc") => Some(vec!["price:asc".to_string()]),
            Some("price_desc") => Some(vec!["price:desc".to_string()]),
            Some("date_desc") => Some(vec!["created_at:desc".to_string()]),
            _ => None, // MeiliSearch defaults to relevance sort
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
        let query = filters.q.as_deref().unwrap_or("");
        let filter_string = Self::build_filter(filters);
        let sort = Self::build_sort(filters.sort.as_deref());

        let mut search = index.search().with_query(query);

        // Apply filter
        if !filter_string.is_empty() {
            search = search.with_filter(filter_string.as_str());
        }

        // Apply sort
        if let Some(ref sort_vec) = sort {
            search = search.with_sort(sort_vec.as_slice());
        }

        // Apply pagination
        let limit = filters.limit() as usize;
        let offset = filters.offset() as usize;
        search = search.with_limit(limit).with_offset(offset);

        // Execute search
        let results: SearchResults<Value> = search
            .execute()
            .await
            .map_err(|e| SearchError::MeiliSearchError(format!("Search query failed: {e}")))?;

        // Map hits to SearchResult
        let items: Vec<SearchResult> = results
            .hits
            .iter()
            .filter_map(|hit| {
                let doc = &hit.result;
                let id = doc.get("id")?.as_str()?;
                let id = Uuid::parse_str(id).ok()?;
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

                // Calculate distance from _geo field if present
                // MeiliSearch returns distance in meters; convert to km.
                let distance_km = hit
                    .formatted_option
                    .as_ref()
                    .and_then(|f| f.get("_geo"))
                    .and_then(|geo| {
                        geo.get("distance")
                            .and_then(|d| d.as_f64())
                            .map(|d| d / 1000.0) // m → km
                    });

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

        let total = results.estimated_total.unwrap_or(items.len() as i64);

        Ok((items, total))
    }

    async fn index_listing(&self, doc: &ListingDoc) -> Result<(), SearchError> {
        let index = self.client.index(LISTINGS_INDEX);

        // Clone the doc to pass an owned value to add_documents
        index
            .add_documents(vec![doc.clone()], Some("id"))
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
