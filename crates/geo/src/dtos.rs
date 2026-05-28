use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Query params para la búsqueda geográfica.
#[derive(Debug, Deserialize)]
pub struct GeoSearchQuery {
    pub lat: f64,
    pub lng: f64,
    /// Radio en metros (máx. 50000 = 50 km)
    pub radius: u32,
    /// Límite de resultados (máx. 100)
    pub limit: Option<u32>,
}

/// DTO de respuesta para un anuncio con distancia.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoListingDto {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: rust_decimal::Decimal,
    pub currency: String,
    pub category: String,
    pub condition: String,
    pub city: Option<String>,
    pub seller_id: Uuid,
    pub distance_m: f64,
    pub created_at: DateTime<Utc>,
}

impl From<crate::models::GeoListing> for GeoListingDto {
    fn from(l: crate::models::GeoListing) -> Self {
        Self {
            id: l.id,
            title: l.title,
            description: l.description,
            price: l.price,
            currency: l.currency,
            category: l.category,
            condition: l.condition,
            city: l.city,
            seller_id: l.seller_id,
            distance_m: l.distance_m,
            created_at: l.created_at,
        }
    }
}

/// DTO de respuesta para la búsqueda geográfica.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoSearchResponse {
    pub data: Vec<GeoListingDto>,
    pub total: usize,
}

impl GeoSearchResponse {
    pub fn new(data: Vec<crate::models::GeoListing>) -> Self {
        let total = data.len();
        Self {
            data: data.into_iter().map(GeoListingDto::from).collect(),
            total,
        }
    }
}
