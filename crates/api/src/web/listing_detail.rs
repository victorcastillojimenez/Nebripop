use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::{State, Path}, response::Html};
use crate::app_state::AppState;
use users::dtos::{UserDto, PublicProfileDto};
use listings::dtos::ListingResponseDto;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "pages/listing_detail.html")]
pub struct ListingDetailTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub listing: ListingResponseDto,
    pub seller: PublicProfileDto,
    pub query_param: Option<String>,
}

pub async fn listing_detail_handler(
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let mock_listing = ListingResponseDto {
        id,
        title: "Producto de ejemplo".to_string(),
        description: "Esta es una descripción de ejemplo para un producto en Nebripop.".to_string(),
        price: 99.99,
        category: "tecnologia".to_string(),
        condition: "nuevo".to_string(),
        image_url: None,
        seller_id: Uuid::new_v4(),
        created_at: chrono::Utc::now(),
        is_favorite: false,
    };

    let mock_seller = PublicProfileDto {
        id: mock_listing.seller_id,
        display_name: "Vendedor Pro".to_string(),
        avatar_url: None,
        rating_avg: 4.8,
        total_ratings: 12,
        created_at: chrono::Utc::now(),
    };

    let template = ListingDetailTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        listing: mock_listing,
        seller: mock_seller,
        query_param: None,
    };
    Html(template.render().unwrap())
}
