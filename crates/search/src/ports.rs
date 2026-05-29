use async_trait::async_trait;
use uuid::Uuid;

use crate::errors::SearchError;
use crate::models::{ListingDoc, SearchFilters, SearchResult};

/// Primary port for search engine operations.
/// Defined in the domain so usecases depend on this trait,
/// not on concrete infrastructure (MeiliSearch or SQL).
#[async_trait]
pub trait SearchEngine: Send + Sync {
    /// Execute a search with the given filters.
    /// Returns a list of search results and the total count.
    async fn search(
        &self,
        filters: &SearchFilters,
    ) -> Result<(Vec<SearchResult>, i64), SearchError>;

    /// Index (create or update) a listing in the search engine.
    async fn index_listing(&self, doc: &ListingDoc) -> Result<(), SearchError>;

    /// Remove a listing from the search engine by ID.
    async fn remove_listing(&self, id: Uuid) -> Result<(), SearchError>;
}
