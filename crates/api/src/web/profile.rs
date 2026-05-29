use askama::Template;
use axum::{extract::{State, Path}, response::Html, http::StatusCode};
use crate::app_state::AppState;
use users::dtos::{UserDto, PublicProfileDto};
use users::ports::UserRepositoryPort;
use listings::dtos::ListingSummaryDto;
use listings::ports::ListingRepository;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use crate::web::filters;

#[derive(Template)]
#[template(path = "users/profile.html")]
pub struct ProfileTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub user: PublicProfileDto,
    pub user_listings: Vec<ListingSummaryDto>,
    pub is_own_profile: bool,
    pub ratings: Vec<MockRating>,
    pub query_param: Option<String>,
}

pub struct MockRating {
    pub reviewer_name: String,
    pub created_at: DateTime<Utc>,
    pub score: i32,
    pub comment: String,
}

pub async fn profile_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let user = state.user_repo.find_by_id(id).await
        .map_err(|e| {
            tracing::error!("Error fetching user profile: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let user_dto = PublicProfileDto {
        id: user.id,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        rating_avg: user.rating_avg.and_then(|d| d.to_f64()).unwrap_or(0.0),
        total_ratings: user.total_ratings,
        created_at: user.created_at,
    };

    let (listings, _) = state.listing_repo.find_by_seller(id, 0, 50).await
        .map_err(|e| {
            tracing::error!("Error fetching listings for user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let user_listings = listings
        .iter()
        .map(ListingSummaryDto::from_listing)
        .collect();

    let template = ProfileTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        user: user_dto,
        user_listings,
        is_own_profile: false,
        ratings: vec![],
        query_param: None,
    };

    template.render()
        .map(Html)
        .map_err(|e| {
            tracing::error!("Failed to render profile template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
