use thiserror::Error;

/// Domain errors for the search module.
#[derive(Debug, Error)]
pub enum SearchError {
    /// MeiliSearch connection or query error.
    #[error("Error de MeiliSearch: {0}")]
    MeiliSearchError(String),

    /// Database query error in SQL fallback.
    #[error("Error de base de datos: {0}")]
    DatabaseError(String),

    /// Invalid search parameters (validation).
    #[error("Parámetros de búsqueda inválidos: {0}")]
    InvalidParams(String),

    /// Invalid search filter parameters.
    #[error("Filtros de búsqueda inválidos: {0}")]
    InvalidFilters(String),

    /// Index setup error.
    #[error("Error de configuración del índice: {0}")]
    IndexSetup(String),

    /// Internal/search engine error.
    #[error("Error interno: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for SearchError {
    fn from(e: sqlx::Error) -> Self {
        SearchError::DatabaseError(e.to_string())
    }
}

impl From<SearchError> for common::errors::AppError {
    fn from(e: SearchError) -> Self {
        match e {
            SearchError::InvalidParams(msg) => common::errors::AppError::BadRequest(msg),
            SearchError::InvalidFilters(msg) => common::errors::AppError::BadRequest(msg),
            SearchError::MeiliSearchError(msg) => {
                tracing::warn!("MeiliSearch error: {}", msg);
                common::errors::AppError::Internal(format!("Error del motor de búsqueda: {msg}"))
            }
            SearchError::DatabaseError(msg) => {
                tracing::error!("Database error during search: {}", msg);
                common::errors::AppError::Internal("Error al realizar la búsqueda".to_string())
            }
            SearchError::IndexSetup(msg) => {
                tracing::error!("Search index setup error: {}", msg);
                common::errors::AppError::Internal("Error de configuración del motor de búsqueda".to_string())
            }
            SearchError::Internal(msg) => {
                tracing::error!("Internal search error: {}", msg);
                common::errors::AppError::Internal(msg)
            }
            
        }
    }
}
impl From<meilisearch_sdk::errors::Error> for SearchError {
    fn from(e: meilisearch_sdk::errors::Error) -> Self {
        SearchError::MeiliSearchError(e.to_string())
    }
}