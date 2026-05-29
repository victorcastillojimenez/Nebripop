use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::{State, Path}, response::Html};
use crate::app_state::AppState;
use users::dtos::{UserDto, PublicProfileDto};
use listings::dtos::ListingResponseDto;
use uuid::Uuid;
use std::str::FromStr;
use crate::web::filters;

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
        seller_id: Uuid::new_v4(),
        title: "Producto de ejemplo".to_string(),
        description: "Esta es una descripción de ejemplo para un producto en Nebripop.".to_string(),
        price: rust_decimal::Decimal::from_str("99.99").unwrap(),
        currency: "EUR".to_string(),
        category: "tecnologia".to_string(),
        condition: listings::models::PhysicalCondition::Used,
        status: listings::models::ListingStatus::Active,
        location_lat: 40.4168,
        location_lon: -3.7038,
        city: "Madrid".to_string(),
        images: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
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

