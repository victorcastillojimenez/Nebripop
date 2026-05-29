use askama::Template;
use axum::{extract::{State, Path}, response::Html, http::StatusCode};
use axum_extra::extract::CookieJar;
use crate::app_state::AppState;
use users::dtos::{UserDto, PublicProfileDto};
use users::ports::UserRepositoryPort;
use listings::dtos::ListingResponseDto;
use listings::ports::ListingRepository;
use uuid::Uuid;
use rust_decimal::prelude::ToPrimitive;
use crate::web::filters;
use common::auth::AuthUser;

#[derive(Template)]
#[template(path = "pages/listing_detail.html")]
pub struct ListingDetailTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub listing: ListingResponseDto,
    pub seller: PublicProfileDto,
    pub query_param: Option<String>,
    pub session_token: String,
}

impl ListingDetailTemplate {
    pub fn is_seller(&self) -> bool {
        self.current_user.as_ref().map_or(false, |u| u.id == self.listing.seller_id)
    }
}

pub async fn listing_detail_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let current_user = crate::web::get_current_user(auth, &state).await;

    let session_token = jar
        .get("session_token")
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    let listing = state.listing_repo.find_by_id(id).await
        .map_err(|e| {
            tracing::error!("Error fetching listing: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let seller = state.user_repo.find_by_id(listing.seller_id).await
        .map_err(|e| {
            tracing::error!("Error fetching seller: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let listing_dto = ListingResponseDto::from_listing(listing);
    let seller_dto = PublicProfileDto {
        id: seller.id,
        display_name: seller.display_name,
        avatar_url: seller.avatar_url,
        rating_avg: seller.rating_avg.and_then(|d| d.to_f64()).unwrap_or(0.0),
        total_ratings: seller.total_ratings,
        created_at: seller.created_at,
    };

    let template = ListingDetailTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        listing: listing_dto,
        seller: seller_dto,
        query_param: None,
        session_token,
    };

    template.render()
        .map(Html)
        .map_err(|e| {
            tracing::error!("Failed to render template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

