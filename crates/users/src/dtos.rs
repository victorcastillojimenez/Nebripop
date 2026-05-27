use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterDto {
    #[validate(email(message = "Email no válido"))]
    pub email: String,

    #[validate(length(
        min = 8,
        message = "La contraseña debe tener al menos 8 caracteres"
    ))]
    pub password: String,

    #[validate(length(
        min = 2,
        message = "El nombre debe tener al menos 2 caracteres"
    ))]
    pub display_name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginDto {
    #[validate(email(message = "Email no válido"))]
    pub email: String,

    #[validate(length(
        min = 1,
        message = "La contraseña es obligatoria"
    ))]
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshDto {}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub user: UserDto,
}

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub rating_avg: f64,
    pub total_ratings: i32,
    pub created_at: DateTime<Utc>,
}

impl From<crate::models::User> for UserDto {
    fn from(user: crate::models::User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            role: user.role,
            rating_avg: user.rating_avg.map(|d| {
                let f: f64 = d.to_string().parse().unwrap_or(0.0);
                f
            }).unwrap_or(0.0),
            total_ratings: user.total_ratings,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PublicProfileDto {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub rating_avg: f64,
    pub total_ratings: i32,
    pub created_at: DateTime<Utc>,
}

impl From<crate::models::PublicProfile> for PublicProfileDto {
    fn from(profile: crate::models::PublicProfile) -> Self {
        Self {
            id: profile.id,
            display_name: profile.display_name,
            avatar_url: profile.avatar_url,
            rating_avg: profile.rating_avg,
            total_ratings: profile.total_ratings,
            created_at: profile.created_at,
        }
    }
}

/// Token-only response for login/refresh
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i64,
}

/// Generic message response
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}
