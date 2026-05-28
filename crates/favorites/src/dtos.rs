use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// DTO de respuesta para un favorito con datos del anuncio embebidos.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteDto {
    pub id: Uuid,
    pub user_id: Uuid,
    pub listing_id: Uuid,
    pub listing_title: Option<String>,
    pub listing_price: Option<rust_decimal::Decimal>,
    pub listing_image_url: Option<String>,
    pub listing_city: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// DTO de respuesta para listado paginado de favoritos.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesListDto {
    pub data: Vec<FavoriteDto>,
    pub total: i64,
}

impl FavoritesListDto {
    pub fn new(data: Vec<FavoriteDto>, total: i64) -> Self {
        Self { data, total }
    }
}

/// DTO de respuesta para añadir favorito (indica si ya existía).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddFavoriteResponse {
    pub added: bool,
    pub already_existed: bool,
}

impl AddFavoriteResponse {
    pub fn new(added: bool, already_existed: bool) -> Self {
        Self {
            added,
            already_existed,
        }
    }
}
