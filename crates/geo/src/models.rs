use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Punto geográfico con latitud y longitud.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lng: f64,
}

impl GeoPoint {
    pub fn new(lat: f64, lng: f64) -> Self {
        Self { lat, lng }
    }

    /// Valida que las coordenadas estén en rangos válidos.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(-90.0..=90.0).contains(&self.lat) {
            return Err("Latitud debe estar entre -90 y 90");
        }
        if !(-180.0..=180.0).contains(&self.lng) {
            return Err("Longitud debe estar entre -180 y 180");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_point_validate_valid() {
        let point = GeoPoint::new(40.4168, -3.7038);
        assert!(point.validate().is_ok());
    }

    #[test]
    fn test_geo_point_validate_lat_out_of_range() {
        let point = GeoPoint::new(91.0, 0.0);
        assert!(point.validate().is_err());
    }

    #[test]
    fn test_geo_point_validate_lat_below_range() {
        let point = GeoPoint::new(-91.0, 0.0);
        assert!(point.validate().is_err());
    }

    #[test]
    fn test_geo_point_validate_lng_out_of_range() {
        let point = GeoPoint::new(0.0, 181.0);
        assert!(point.validate().is_err());
    }

    #[test]
    fn test_geo_point_validate_lng_below_range() {
        let point = GeoPoint::new(0.0, -181.0);
        assert!(point.validate().is_err());
    }

    #[test]
    fn test_geo_point_validate_boundary_lat() {
        assert!(GeoPoint::new(90.0, 0.0).validate().is_ok());
        assert!(GeoPoint::new(-90.0, 0.0).validate().is_ok());
    }

    #[test]
    fn test_geo_point_validate_boundary_lng() {
        assert!(GeoPoint::new(0.0, 180.0).validate().is_ok());
        assert!(GeoPoint::new(0.0, -180.0).validate().is_ok());
    }
}

/// Resultado de búsqueda geográfica con distancia incluida.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoListing {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: rust_decimal::Decimal,
    pub currency: String,
    pub category: String,
    pub condition: String,
    pub status: String,
    pub city: Option<String>,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
    pub distance_m: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
