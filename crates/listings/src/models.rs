use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Strongly-typed identifier for a Listing.
/// Used in domain logic to avoid confusion with raw Uuid values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ListingId(pub Uuid);

impl std::fmt::Display for ListingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ListingId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<ListingId> for Uuid {
    fn from(id: ListingId) -> Self {
        id.0
    }
}

/// Physical condition of the item being listed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalCondition {
    New,
    LikeNew,
    Used,
}

impl PhysicalCondition {
    pub fn as_str(&self) -> &'static str {
        match self {
            PhysicalCondition::New => "new",
            PhysicalCondition::LikeNew => "like_new",
            PhysicalCondition::Used => "used",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "new" => Some(PhysicalCondition::New),
            "like_new" | "like-new" => Some(PhysicalCondition::LikeNew),
            "used" => Some(PhysicalCondition::Used),
            _ => None,
        }
    }
}

/// Logical status of a listing in the system lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ListingStatus {
    Active,
    Sold,
    Deleted,
}

impl ListingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ListingStatus::Active => "active",
            ListingStatus::Sold => "sold",
            ListingStatus::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(ListingStatus::Active),
            "sold" => Some(ListingStatus::Sold),
            "deleted" => Some(ListingStatus::Deleted),
            _ => None,
        }
    }
}

/// Domain entity representing a marketplace listing.
/// NOTE: This struct intentionally does NOT derive sqlx::FromRow.
/// Database mapping is handled via a private row type in the adapter layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
    pub id: ListingId,
    pub seller_id: Uuid,
    pub title: String,
    pub description: String,
    pub price: Decimal,
    pub currency: String,
    pub category: String,
    pub condition: PhysicalCondition,
    pub status: ListingStatus,
    pub location_lat: f64,
    pub location_lon: f64,
    pub city: String,
    pub images: Vec<ListingImage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Domain entity representing an image attached to a listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingImage {
    pub id: ListingImageId,
    pub listing_id: ListingId,
    pub image_url: String,
    pub position: i32,
}

/// Strongly-typed identifier for a ListingImage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ListingImageId(pub Uuid);

impl std::fmt::Display for ListingImageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ListingImageId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<ListingImageId> for Uuid {
    fn from(id: ListingImageId) -> Self {
        id.0
    }
}

/// Builder-like constructor for Listing.
impl Listing {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ListingId,
        seller_id: Uuid,
        title: String,
        description: String,
        price: Decimal,
        currency: String,
        category: String,
        condition: PhysicalCondition,
        location_lat: f64,
        location_lon: f64,
        city: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            seller_id,
            title,
            description,
            price,
            currency,
            category,
            condition,
            status: ListingStatus::Active,
            location_lat,
            location_lon,
            city,
            images: Vec::new(),
            created_at,
            updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listing_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = ListingId::from(uuid);
        assert_eq!(id.0, uuid);
    }

    #[test]
    fn test_listing_id_into_uuid() {
        let uuid = Uuid::new_v4();
        let id = ListingId(uuid);
        let back: Uuid = id.into();
        assert_eq!(back, uuid);
    }

    #[test]
    fn test_physical_condition_roundtrip() {
        for (variant, s) in &[
            (PhysicalCondition::New, "new"),
            (PhysicalCondition::LikeNew, "like_new"),
            (PhysicalCondition::Used, "used"),
        ] {
            assert_eq!(variant.as_str(), *s);
            assert_eq!(PhysicalCondition::from_str(s), Some(*variant));
        }
        assert_eq!(PhysicalCondition::from_str("unknown"), None);
    }

    #[test]
    fn test_listing_status_roundtrip() {
        for (variant, s) in &[
            (ListingStatus::Active, "active"),
            (ListingStatus::Sold, "sold"),
            (ListingStatus::Deleted, "deleted"),
        ] {
            assert_eq!(variant.as_str(), *s);
            assert_eq!(ListingStatus::from_str(s), Some(*variant));
        }
        assert_eq!(ListingStatus::from_str("unknown"), None);
    }

    #[test]
    fn test_listing_new_defaults_active() {
        let listing = Listing::new(
            ListingId(Uuid::new_v4()),
            Uuid::new_v4(),
            "Test".to_string(),
            "Description".to_string(),
            Decimal::new(1000, 2),
            "eur".to_string(),
            "electronics".to_string(),
            PhysicalCondition::New,
            40.4168,
            -3.7038,
            "Madrid".to_string(),
            Utc::now(),
            Utc::now(),
        );
        assert_eq!(listing.status, ListingStatus::Active);
        assert!(listing.images.is_empty());
    }
}
