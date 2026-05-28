use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::GeoError;
use crate::models::GeoListing;

#[derive(Debug, Clone)]
pub struct GeoRepository {
    pool: PgPool,
}

impl GeoRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Busca anuncios activos cercanos a una ubicación usando PostGIS.
    ///
    /// Utiliza ST_DWithin con GEOGRAPHY para búsqueda esférica precisa,
    /// y ST_Distance para calcular la distancia exacta en metros.
    ///
    /// # Parámetros
    /// - `lat`: Latitud del punto de origen
    /// - `lng`: Longitud del punto de origen
    /// - `radius_m`: Radio de búsqueda en metros (máx. 50000)
    /// - `max_limit`: Número máximo de resultados (máx. 100)
    pub async fn search_nearby(
        &self,
        lat: f64,
        lng: f64,
        radius_m: f64,
        max_limit: i64,
    ) -> Result<Vec<GeoListing>, GeoError> {
        // Usamos query raw (no query_as! con macro) porque PostGIS y sus funciones
        // pueden no estar disponibles en tiempo de compilación si no hay BD conectada.
        let rows = sqlx::query_as::<_, GeoListingRow>(
            r#"SELECT
                l.id,
                l.seller_id,
                l.title,
                l.description,
                l.price,
                l.currency,
                l.category,
                l.condition,
                l.status,
                l.city,
                l.location_lat,
                l.location_lon,
                ST_Distance(l.location, ST_MakePoint($1, $2)::geography) AS distance_m,
                l.created_at,
                l.updated_at
               FROM listings l
               WHERE l.status = 'active'
                 AND l.location IS NOT NULL
                 AND ST_DWithin(l.location, ST_MakePoint($1, $2)::geography, $3)
               ORDER BY distance_m ASC
               LIMIT $4"#,
        )
        .bind(lng)   // ST_MakePoint(lon, lat) — PostGIS usa (X=lon, Y=lat)
        .bind(lat)
        .bind(radius_m)
        .bind(max_limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in search_nearby: {}", e);
            GeoError::DatabaseError(e.to_string())
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Obtiene el pool (para transacciones).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Struct interno para mapear el resultado raw de la query PostGIS.
#[derive(Debug, sqlx::FromRow)]
struct GeoListingRow {
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<GeoListingRow> for GeoListing {
    fn from(r: GeoListingRow) -> Self {
        Self {
            id: r.id,
            seller_id: r.seller_id,
            title: r.title,
            description: r.description,
            price: r.price,
            currency: r.currency,
            category: r.category,
            condition: r.condition,
            status: r.status,
            city: r.city,
            location_lat: r.location_lat,
            location_lon: r.location_lon,
            distance_m: r.distance_m,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
