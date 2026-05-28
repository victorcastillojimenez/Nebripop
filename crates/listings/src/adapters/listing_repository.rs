use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::ListingError;
use crate::models::{
    Listing, ListingId, ListingImage, ListingImageId, ListingStatus, PhysicalCondition,
};
use crate::ports::ListingRepository;

/// Private database row struct with sqlx::FromRow.
/// Never exposed outside the adapter — domain entities remain decoupled.
#[derive(Debug, sqlx::FromRow)]
struct ListingRow {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub title: String,
    pub description: String,
    pub price: Decimal,
    pub currency: String,
    pub category: String,
    pub condition: String,
    pub status: String,
    pub location_lat: f64,
    pub location_lon: f64,
    pub city: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Private database row for listing images.
#[derive(Debug, sqlx::FromRow)]
struct ListingImageRow {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub image_url: String,
    pub position: i32,
}

/// Converts a DB row to domain Listing.
impl TryFrom<ListingRow> for Listing {
    type Error = ListingError;

    fn try_from(row: ListingRow) -> Result<Self, Self::Error> {
        let condition = PhysicalCondition::from_str(&row.condition)
            .ok_or_else(|| ListingError::InvalidInput(format!("Unknown condition: {}", row.condition)))?;

        let status = ListingStatus::from_str(&row.status)
            .ok_or_else(|| ListingError::InvalidInput(format!("Unknown status: {}", row.status)))?;

        Ok(Self {
            id: ListingId(row.id),
            seller_id: row.seller_id,
            title: row.title,
            description: row.description,
            price: row.price,
            currency: row.currency,
            category: row.category,
            condition,
            status,
            location_lat: row.location_lat,
            location_lon: row.location_lon,
            city: row.city.unwrap_or_default(),
            images: Vec::new(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl From<ListingImageRow> for ListingImage {
    fn from(row: ListingImageRow) -> Self {
        Self {
            id: ListingImageId(row.id),
            listing_id: ListingId(row.listing_id),
            image_url: row.image_url,
            position: row.position,
        }
    }
}

/// Concrete implementation of ListingRepository using SQLx + PostgreSQL.
#[derive(Debug, Clone)]
pub struct ListingRepositoryImpl {
    pool: PgPool,
}

impl ListingRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ListingRepository for ListingRepositoryImpl {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Listing>, ListingError> {
        let row: Option<ListingRow> = sqlx::query_as::<_, ListingRow>(
            r#"SELECT id, seller_id, title, description, price, currency,
                      category, condition, status, location_lat, location_lon,
                      city, created_at, updated_at
               FROM listings
               WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in find_by_id: {}", e);
            ListingError::Database(e)
        })?;

        match row {
            Some(r) => {
                let images = self.find_images_by_listing(id).await?;
                let mut listing: Listing = r.try_into()?;
                listing.images = images;
                Ok(Some(listing))
            }
            None => Ok(None),
        }
    }

    async fn find_all_paginated(
        &self,
        page: i64,
        per_page: i64,
        category: Option<&str>,
    ) -> Result<(Vec<Listing>, i64), ListingError> {
        let offset = page * per_page;

        // Count total (always filtered by active status)
        let total: (i64,) = if let Some(cat) = category {
            sqlx::query_as(
                r#"SELECT COUNT(*)::int8 FROM listings WHERE status = 'active' AND category = $1"#,
            )
            .bind(cat)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error in count listings: {}", e);
                ListingError::Database(e)
            })?
        } else {
            sqlx::query_as(
                r#"SELECT COUNT(*)::int8 FROM listings WHERE status = 'active'"#,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error in count listings: {}", e);
                ListingError::Database(e)
            })?
        };

        // Fetch listings
        let rows: Vec<ListingRow> = if let Some(cat) = category {
            sqlx::query_as::<_, ListingRow>(
                r#"SELECT id, seller_id, title, description, price, currency,
                          category, condition, status, location_lat, location_lon,
                          city, created_at, updated_at
                   FROM listings
                   WHERE status = 'active' AND category = $1
                   ORDER BY created_at DESC
                   LIMIT $2 OFFSET $3"#,
            )
            .bind(cat)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error in find_all_paginated: {}", e);
                ListingError::Database(e)
            })?
        } else {
            sqlx::query_as::<_, ListingRow>(
                r#"SELECT id, seller_id, title, description, price, currency,
                          category, condition, status, location_lat, location_lon,
                          city, created_at, updated_at
                   FROM listings
                   WHERE status = 'active'
                   ORDER BY created_at DESC
                   LIMIT $1 OFFSET $2"#,
            )
            .bind(per_page)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error in find_all_paginated: {}", e);
                ListingError::Database(e)
            })?
        };

        // Enrich each listing with its images
        let mut listings = Vec::with_capacity(rows.len());
        for row in rows {
            let mut listing: Listing = row.try_into()?;
            listing.images = self.find_images_by_listing(listing.id.0).await?;
            listings.push(listing);
        }

        Ok((listings, total.0))
    }

    async fn find_by_seller(
        &self,
        seller_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Listing>, i64), ListingError> {
        let offset = page * per_page;

        let total: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)::int8 FROM listings WHERE seller_id = $1"#,
        )
        .bind(seller_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in count by seller: {}", e);
            ListingError::Database(e)
        })?;

        let rows: Vec<ListingRow> = sqlx::query_as::<_, ListingRow>(
            r#"SELECT id, seller_id, title, description, price, currency,
                      category, condition, status, location_lat, location_lon,
                      city, created_at, updated_at
               FROM listings
               WHERE seller_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(seller_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in find_by_seller: {}", e);
            ListingError::Database(e)
        })?;

        let mut listings = Vec::with_capacity(rows.len());
        for row in rows {
            let mut listing: Listing = row.try_into()?;
            listing.images = self.find_images_by_listing(listing.id.0).await?;
            listings.push(listing);
        }

        Ok((listings, total.0))
    }

    async fn insert(
        &self,
        id: Uuid,
        seller_id: Uuid,
        title: &str,
        description: &str,
        price: Decimal,
        currency: &str,
        category: &str,
        condition: &PhysicalCondition,
        location_lat: f64,
        location_lon: f64,
        city: &str,
    ) -> Result<Listing, ListingError> {
        let condition_str = condition.as_str();

        let row: ListingRow = sqlx::query_as::<_, ListingRow>(
            r#"INSERT INTO listings (id, seller_id, title, description, price, currency,
                                      category, condition, location_lat, location_lon, city)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
               RETURNING id, seller_id, title, description, price, currency,
                         category, condition, status, location_lat, location_lon,
                         city, created_at, updated_at"#,
        )
        .bind(id)
        .bind(seller_id)
        .bind(title)
        .bind(description)
        .bind(price)
        .bind(currency)
        .bind(category)
        .bind(condition_str)
        .bind(location_lat)
        .bind(location_lon)
        .bind(city)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in insert listing: {}", e);
            ListingError::Database(e)
        })?;

        let listing: Listing = row.try_into()?;
        Ok(listing)
    }

    async fn update(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        price: Option<Decimal>,
        category: Option<&str>,
        condition: Option<&PhysicalCondition>,
        location_lat: Option<f64>,
        location_lon: Option<f64>,
        city: Option<&str>,
    ) -> Result<Listing, ListingError> {
        // First, get existing listing to merge
        let existing = self.find_by_id(id).await?;
        let existing = existing.ok_or(ListingError::NotFound(id))?;

        let new_title = title.unwrap_or(&existing.title);
        let new_description = description.unwrap_or(&existing.description);
        let new_price = price.unwrap_or(existing.price);
        let new_category = category.unwrap_or(&existing.category);
        let new_condition = condition.map(|c| c.as_str()).unwrap_or_else(|| existing.condition.as_str());
        let new_lat = location_lat.unwrap_or(existing.location_lat);
        let new_lon = location_lon.unwrap_or(existing.location_lon);
        let new_city = city.unwrap_or(&existing.city);

        let row: ListingRow = sqlx::query_as::<_, ListingRow>(
            r#"UPDATE listings
               SET title = $2, description = $3, price = $4, category = $5,
                   condition = $6, location_lat = $7, location_lon = $8,
                   city = $9, updated_at = now()
               WHERE id = $1
               RETURNING id, seller_id, title, description, price, currency,
                         category, condition, status, location_lat, location_lon,
                         city, created_at, updated_at"#,
        )
        .bind(id)
        .bind(new_title)
        .bind(new_description)
        .bind(new_price)
        .bind(new_category)
        .bind(new_condition)
        .bind(new_lat)
        .bind(new_lon)
        .bind(new_city)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in update listing: {}", e);
            ListingError::Database(e)
        })?;

        let mut listing: Listing = row.try_into()?;
        listing.images = self.find_images_by_listing(id).await?;
        Ok(listing)
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), ListingError> {
        let result = sqlx::query(
            r#"UPDATE listings SET status = 'deleted', updated_at = now() WHERE id = $1"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in soft_delete: {}", e);
            ListingError::Database(e)
        })?;

        if result.rows_affected() == 0 {
            return Err(ListingError::NotFound(id));
        }
        Ok(())
    }

    async fn mark_as_sold(&self, id: Uuid) -> Result<(), ListingError> {
        let result = sqlx::query(
            r#"UPDATE listings SET status = 'sold', updated_at = now() WHERE id = $1"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in mark_as_sold: {}", e);
            ListingError::Database(e)
        })?;

        if result.rows_affected() == 0 {
            return Err(ListingError::NotFound(id));
        }
        Ok(())
    }

    async fn count_images(&self, listing_id: Uuid) -> Result<i32, ListingError> {
        let count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)::int8 FROM listing_images WHERE listing_id = $1"#,
        )
        .bind(listing_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in count_images: {}", e);
            ListingError::Database(e)
        })?;

        Ok(count.0 as i32)
    }

    async fn insert_image(
        &self,
        id: Uuid,
        listing_id: Uuid,
        image_url: &str,
        position: i32,
    ) -> Result<ListingImage, ListingError> {
        let row: ListingImageRow = sqlx::query_as::<_, ListingImageRow>(
            r#"INSERT INTO listing_images (id, listing_id, image_url, position)
               VALUES ($1, $2, $3, $4)
               RETURNING id, listing_id, image_url, position"#,
        )
        .bind(id)
        .bind(listing_id)
        .bind(image_url)
        .bind(position)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in insert_image: {}", e);
            ListingError::Database(e)
        })?;

        Ok(ListingImage::from(row))
    }

    async fn find_images_by_listing(&self, listing_id: Uuid) -> Result<Vec<ListingImage>, ListingError> {
        let rows: Vec<ListingImageRow> = sqlx::query_as::<_, ListingImageRow>(
            r#"SELECT id, listing_id, image_url, position
               FROM listing_images
               WHERE listing_id = $1
               ORDER BY position ASC"#,
        )
        .bind(listing_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in find_images_by_listing: {}", e);
            ListingError::Database(e)
        })?;

        Ok(rows.into_iter().map(ListingImage::from).collect())
    }

    async fn delete_images_by_listing(&self, listing_id: Uuid) -> Result<(), ListingError> {
        sqlx::query(r#"DELETE FROM listing_images WHERE listing_id = $1"#)
            .bind(listing_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error in delete_images_by_listing: {}", e);
                ListingError::Database(e)
            })?;

        Ok(())
    }
}
