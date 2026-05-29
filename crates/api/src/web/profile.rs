use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::{State, Path}, response::Html};
use crate::app_state::AppState;
use users::dtos::{UserDto, PublicProfileDto};
use listings::dtos::ListingSummaryDto;
use uuid::Uuid;
use chrono::{DateTime, Utc};
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
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> impl IntoResponse {
    let mock_profile = PublicProfileDto {
        id: _id,
        display_name: "Usuario Nebrija".to_string(),
        avatar_url: None,
        rating_avg: 5.0,
        total_ratings: 0,
        created_at: Utc::now(),
    };

    let template = ProfileTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        user: mock_profile,
        user_listings: vec![],
        is_own_profile: false,
        ratings: vec![],
        query_param: None,
    };
    Html(template.render().unwrap())
}
