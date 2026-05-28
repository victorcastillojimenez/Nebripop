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
    pub fn new(url: &str, api_key: Option<&str>) -> Self {
        let client = match api_key {
            Some(key) => Client::new(url, Some(key)),
            None => Client::new(url, None::<&str>),
        };
        Self { client: client.expect("Failed to create Meilisearch client") }
    }

    async fn get_or_create_index(&self) -> Result<Index, SearchError> {
        let task = self
            .client
            .create_index(LISTINGS_INDEX, Some("id"))
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to create index: {e}")))?;

        // Usar wait_for_task con el Task completo, no con el uid
        let _ = self
            .client
            .wait_for_task(
                task,
                Some(std::time::Duration::from_millis(500)),
                Some(std::time::Duration::from_secs(5)),
            )
            .await
            .ok();

        Ok(self.client.index(LISTINGS_INDEX))
    }

    pub async fn setup_index(&self) -> Result<(), SearchError> {
        let index = self.get_or_create_index().await?;

        index
            .set_searchable_attributes(["title", "description", "city"])
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to set searchable attributes: {e}")))?;

        index
            .set_filterable_attributes(["category", "price", "status", "_geo"])
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to set filterable attributes: {e}")))?;

        index
            .set_sortable_attributes(["price", "created_at"])
            .await
            .map_err(|e| SearchError::IndexSetup(format!("Failed to set sortable attributes: {e}")))?;

        tracing::info!("MeiliSearch index '{}' configured successfully", LISTINGS_INDEX);

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
            parts.push(format!("price >= {}", min));
        }
        if let Some(max) = filters.max_price {
            parts.push(format!("price <= {}", max));
        }

        if let (Some(lat), Some(lng)) = (filters.lat, filters.lng) {
            let radius_m = filters
                .radius_km
                .unwrap_or(50.0)
                .max(1.0)
                * 1000.0;
            parts.push(format!("_geoRadius({}, {}, {})", lat, lng, radius_m));
        }

        parts.join(" AND ")
    }

    fn build_sort(sort: Option<&str>) -> Option<Vec<String>> {
        match sort {
            Some("price_asc")  => Some(vec!["price:asc".to_string()]),
            Some("price_desc") => Some(vec!["price:desc".to_string()]),
            Some("date_desc")  => Some(vec!["created_at:desc".to_string()]),
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
        let query = filters.q.as_deref().unwrap_or("").to_string();
        let filter_string = Self::build_filter(filters);
        let sort = Self::build_sort(filters.sort.as_deref());
        let limit = filters.limit() as usize;
        let offset = filters.offset() as usize;

        // Construir sort_refs con lifetime suficiente
        let sort_strings: Vec<String> = sort.unwrap_or_default();
        let sort_refs: Vec<&str> = sort_strings.iter().map(|s| s.as_str()).collect();

        // Binding explícito para que filter_string viva suficiente
        let filter_str = filter_string.as_str();
        let mut search = index.search();
        search.query = Some(query.as_str());

        if !filter_string.is_empty() {
            search.with_filter(filter_str);
        }

        if !sort_refs.is_empty() {
            search.with_sort(sort_refs.as_slice());
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
                let category  = doc.get("category")?.as_str()?.to_string();
                let condition = doc.get("condition")?.as_str()?.to_string();
                let city      = doc.get("city")?.as_str()?.to_string();
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