use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub phone: Option<String>,
    pub role: String,
    pub rating_avg: Option<rust_decimal::Decimal>,
    pub total_ratings: i32,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    User,
    Admin,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::User => "user",
            UserRole::Admin => "admin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "user" => Some(UserRole::User),
            "admin" => Some(UserRole::Admin),
            _ => None,
        }
    }
}

/// Public profile that is safe to expose via API (never includes password_hash or email)
#[derive(Debug, Clone, Serialize)]
pub struct PublicProfile {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub rating_avg: f64,
    pub total_ratings: i32,
    pub created_at: DateTime<Utc>,
}
