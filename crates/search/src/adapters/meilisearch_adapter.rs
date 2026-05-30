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
        if self.index_exists().await? {
            return Ok(self.client.index(LISTINGS_INDEX));
        }
        self.create_listings_index().await?;
        Ok(self.client.index(LISTINGS_INDEX))
    }

    async fn index_exists(&self) -> Result<bool, SearchError> {
        let existing = self
            .client
            .list_all_indexes()
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to list indexes: {e}")))?;
        Ok(existing.results.iter().any(|i| i.uid == LISTINGS_INDEX))
    }

    async fn create_listings_index(&self) -> Result<(), SearchError> {
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
        Ok(())
    }

    /// Ensure the index exists with the correct settings (idempotent).
    pub async fn setup_index(&self) -> Result<(), SearchError> {
        let index = self.get_or_create_index().await?;

        index
            .set_searchable_attributes(["title", "description", "city"])
            .await?;

        index
            .set_filterable_attributes(["category", "price", "status", "condition", "_geo"])
            .await?;

        index
            .set_sortable_attributes(["price", "created_at"])
            .await?;

        tracing::info!("MeiliSearch index '{LISTINGS_INDEX}' configured successfully");

        Ok(())
    }

    fn build_filter(filters: &SearchFilters) -> String {
        let mut parts = vec!["status = 'active'".to_string()];

        if let Some(ref cat) = filters.category {
            parts.push(format!("category = '{}'", cat.replace('\'', "\\'")));
        }
        if let Some(ref conditions) = filters.condition {
            if conditions.len() == 1 {
                let escaped = conditions[0].replace('\'', "\\'");
                parts.push(format!("condition = '{}'", escaped));
            } else if conditions.len() > 1 {
                let escaped: Vec<String> = conditions
                    .iter()
                    .map(|c| format!("'{}'", c.replace('\'', "\\'")))
                    .collect();
                parts.push(format!("condition IN [{}]", escaped.join(", ")));
            }
        }
        if let Some(min) = filters.min_price {
            parts.push(format!("price >= {min}"));
        }
        if let Some(max) = filters.max_price {
            parts.push(format!("price <= {max}"));
        }
        if let (Some(lat), Some(lng)) = (filters.latitude, filters.longitude) {
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

fn map_hit_to_result(hit: &meilisearch_sdk::search::SearchResult<Value>) -> Option<SearchResult> {
    let doc = &hit.result;
    let distance_km = hit.formatted_result.as_ref()
        .and_then(|f| f.get("_geo")?.get("distance")?.as_f64())
        .map(|d| d / 1000.0);

    Some(SearchResult {
        id: Uuid::parse_str(doc.get("id")?.as_str()?).ok()?,
        title: doc.get("title")?.as_str()?.to_string(),
        price: doc.get("price")?.as_f64()?,
        currency: doc.get("currency").and_then(|v| v.as_str()).unwrap_or("eur").to_string(),
        category: doc.get("category")?.as_str()?.to_string(),
        condition: doc.get("condition")?.as_str()?.to_string(),
        city: doc.get("city")?.as_str()?.to_string(),
        image_url: doc.get("image_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
        distance_km,
        created_at: doc.get("created_at")?.as_i64()?,
    })
}

#[async_trait]
impl SearchEngine for MeiliSearchAdapter {
    async fn search(
        &self,
        filters: &SearchFilters,
    ) -> Result<(Vec<SearchResult>, i64), SearchError> {
        let index = self.client.index(LISTINGS_INDEX);
        let query_owned = filters.query.as_deref().unwrap_or("").to_string();
        let filter_owned = Self::build_filter(filters);
        let sort_vec = Self::build_sort(filters.sort.as_deref()).unwrap_or_default();

        let mut search = index.search();
        search.with_query(&query_owned);
        if !filter_owned.is_empty() { search.with_filter(&filter_owned); }
        if !sort_vec.is_empty() { search.with_sort(&sort_vec); }
        search.with_limit(filters.limit() as usize);
        search.with_offset(filters.offset() as usize);

        let results = search.execute().await
            .map_err(|e| SearchError::MeiliSearchError(format!("Search failed: {e}")))?;

        let items: Vec<SearchResult> = results.hits.iter().filter_map(map_hit_to_result).collect();
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