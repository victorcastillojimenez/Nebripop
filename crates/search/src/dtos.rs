use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Query parameters for the search endpoint.
/// All fields are optional; defaults are applied in validation.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQueryDto {
    /// Full-text search query (optional).
    #[serde(rename = "q")]
    pub query: Option<String>,

    /// Category filter (optional).
    pub category: Option<String>,

    /// Minimum price (optional, >= 0).
    pub min_price: Option<f64>,

    /// Maximum price (optional, >= 0).
    pub max_price: Option<f64>,

    /// Latitude for geo-radius search (optional, must be paired with lng).
    #[serde(rename = "lat")]
    pub latitude: Option<f64>,

    /// Longitude for geo-radius search (optional, must be paired with lat).
    #[serde(rename = "lng")]
    pub longitude: Option<f64>,

    /// Radius in km for geo filter (optional, default: 50 if lat/lng provided).
    pub radius_km: Option<f64>,

    /// Sort order: "price_asc", "price_desc", "date_desc" (optional).
    pub sort: Option<String>,

    /// Page number (0-indexed, default: 0).
    #[serde(default = "default_page")]
    pub page: i64,

    /// Results per page (default: 20, max: 100).
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    0
}

fn default_per_page() -> i64 {
    20
}

impl Default for SearchQueryDto {
    fn default() -> Self {
        Self {
            query: None,
            category: None,
            min_price: None,
            max_price: None,
            latitude: None,
            longitude: None,
            radius_km: None,
            sort: None,
            page: default_page(),
            per_page: default_per_page(),
        }
    }
}

fn validate_prices(min_price: Option<f64>, max_price: Option<f64>) -> Result<(), String> {
    if let Some(min) = min_price {
        if min < 0.0 {
            return Err("minPrice debe ser mayor o igual a 0".to_string());
        }
    }
    if let Some(max) = max_price {
        if max < 0.0 {
            return Err("maxPrice debe ser mayor o igual a 0".to_string());
        }
    }
    if let (Some(min), Some(max)) = (min_price, max_price) {
        if min > max {
            return Err("minPrice no puede ser mayor que maxPrice".to_string());
        }
    }
    Ok(())
}

fn validate_lat_lng_parity(lat: Option<f64>, lng: Option<f64>) -> Result<(), String> {
    if lat.is_some() && lng.is_none() {
        return Err("Si se proporciona lat, también se debe proporcionar lng".to_string());
    }
    if lng.is_some() && lat.is_none() {
        return Err("Si se proporciona lng, también se debe proporcionar lat".to_string());
    }
    Ok(())
}

fn validate_coordinates(lat: Option<f64>, lng: Option<f64>) -> Result<(), String> {
    if let Some(lat_val) = lat {
        if !(-90.0..=90.0).contains(&lat_val) {
            return Err("lat debe estar entre -90 y 90".to_string());
        }
    }
    if let Some(lng_val) = lng {
        if !(-180.0..=180.0).contains(&lng_val) {
            return Err("lng debe estar entre -180 y 180".to_string());
        }
    }
    Ok(())
}

fn validate_radius(radius_km: Option<f64>) -> Result<(), String> {
    if let Some(radius) = radius_km {
        if radius <= 0.0 {
            return Err("radiusKm debe ser mayor que 0".to_string());
        }
        if radius > 500.0 {
            return Err("radiusKm no puede exceder 500 km".to_string());
        }
    }
    Ok(())
}

fn validate_sort(sort: Option<&str>) -> Result<(), String> {
    if let Some(s) = sort {
        match s {
            "price_asc" | "price_desc" | "date_desc" => {}
            _ => {
                return Err(format!(
                    "sort inválido: '{}'. Valores: price_asc, price_desc, date_desc",
                    s
                ));
            }
        }
    }
    Ok(())
}

impl SearchQueryDto {
    /// Validate the query parameters, returning a normalized SearchQueryDto
    /// or an error message if validation fails.
    pub fn validate(self) -> Result<Self, String> {
        validate_prices(self.min_price, self.max_price)?;
        validate_lat_lng_parity(self.latitude, self.longitude)?;
        validate_coordinates(self.latitude, self.longitude)?;
        validate_radius(self.radius_km)?;
        validate_sort(self.sort.as_deref())?;

        Ok(Self {
            page: self.page.max(0),
            per_page: self.per_page.min(100).max(1),
            ..self
        })
    }
}

/// Single search result item for API responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub id: Uuid,
    pub title: String,
    pub price: f64,
    pub currency: String,
    pub category: String,
    pub condition: String,
    pub city: String,
    pub image_url: Option<String>,
    pub distance_km: Option<f64>,
    pub created_at: i64,
}

impl From<crate::models::SearchResult> for SearchResultDto {
    fn from(r: crate::models::SearchResult) -> Self {
        Self {
            id: r.id,
            title: r.title,
            price: r.price,
            currency: r.currency,
            category: r.category,
            condition: r.condition,
            city: r.city,
            image_url: r.image_url,
            distance_km: r.distance_km,
            created_at: r.created_at,
        }
    }
}

/// Paginated search response with engine info.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponseDto {
    pub items: Vec<SearchResultDto>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub engine: String,
}

impl SearchResponseDto {
    pub fn new(
        items: Vec<SearchResultDto>,
        total: i64,
        page: i64,
        per_page: i64,
        engine: &str,
    ) -> Self {
        let total_pages = compute_total_pages(total, per_page);

        Self {
            items,
            total,
            page,
            per_page,
            total_pages,
            engine: engine.to_string(),
        }
    }
}

fn compute_total_pages(total: i64, per_page: i64) -> i64 {
    if total == 0 {
        0
    } else {
        ((total as f64) / (per_page as f64)).ceil() as i64
    }
}
