use askama::Template;
use axum::{extract::{State, Path}, response::{Html, IntoResponse}, http::StatusCode};
use crate::app_state::AppState;
use users::dtos::{UserDto, PublicProfileDto};
use users::ports::UserRepositoryPort;
use listings::dtos::ListingSummaryDto;
use listings::ports::ListingRepository;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use crate::web::filters;
use common::auth::AuthUser;

use ratings::ports::RatingPort;

#[derive(Template)]
#[template(path = "users/my_profile.html")]
pub struct MyProfileTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub user: PublicProfileDto,
    pub user_listings: Vec<ListingSummaryDto>,
    pub ratings: Vec<MockRating>,
    pub query_param: Option<String>,
}

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
    auth: Option<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let current_user = crate::web::get_current_user(auth, &state).await;

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

    let is_own_profile = current_user.as_ref().map(|u| u.id == id).unwrap_or(false);

    let template = ProfileTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        user: user_dto,
        user_listings,
        is_own_profile,
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

pub async fn profile_me_handler(
    auth: Option<AuthUser>,
) -> impl askama_axum::IntoResponse {
    if let Some(_) = auth {
        axum::response::Redirect::to("/me").into_response()
    } else {
        axum::response::Redirect::to("/login").into_response()
    }
}

pub async fn my_profile_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
) -> impl askama_axum::IntoResponse {
    let auth_user = match auth {
        Some(au) => au,
        None => {
            return axum::response::Redirect::to("/login").into_response();
        }
    };

    let user = match state.user_repo.find_by_id(auth_user.id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return axum::response::Redirect::to("/login").into_response();
        }
        Err(e) => {
            tracing::error!("Error fetching user for my profile: {}", e);
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let user_dto = PublicProfileDto {
        id: user.id,
        display_name: user.display_name.clone(),
        avatar_url: user.avatar_url.clone(),
        rating_avg: user.rating_avg.and_then(|d| d.to_f64()).unwrap_or(0.0),
        total_ratings: user.total_ratings,
        created_at: user.created_at,
    };

    let (listings, _) = match state.listing_repo.find_by_seller(auth_user.id, 0, 50).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Error fetching listings for my profile: {}", e);
            (vec![], 0)
        }
    };

    let user_listings = listings
        .iter()
        .map(ListingSummaryDto::from_listing)
        .collect();

    let ratings_from_db = match state.rating_repo.find_by_user_id(auth_user.id, 0, 50).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Error fetching ratings for my profile: {}", e);
            vec![]
        }
    };

    let mut ratings = Vec::with_capacity(ratings_from_db.len());
    for r in ratings_from_db {
        let rater_name = match state.user_repo.find_by_id(r.rater_id).await {
            Ok(Some(u)) => u.display_name,
            _ => "Usuario de Nebripop".to_string(),
        };
        ratings.push(MockRating {
            reviewer_name: rater_name,
            created_at: r.created_at,
            score: r.score as i32,
            comment: r.comment.unwrap_or_default(),
        });
    }

    let current_user_dto = Some(UserDto::from(user));

    let template = MyProfileTemplate {
        current_user: current_user_dto,
        flash_success: None,
        flash_error: None,
        user: user_dto,
        user_listings,
        ratings,
        query_param: None,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Failed to render my_profile template: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
