use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::State, response::Html};
use axum_extra::extract::CookieJar;
use crate::app_state::AppState;
use users::dtos::UserDto;
use listings::dtos::ListingSummaryDto;
use listings::ports::ListingRepository;
use crate::web::filters;
use common::auth::AuthUser;

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct HomeTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub recent_listings: Vec<ListingSummaryDto>,
    pub query_param: Option<String>,
    pub session_token: String,
}

pub async fn home_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    jar: CookieJar,
) -> impl IntoResponse {
    let current_user = crate::web::get_current_user(auth, &state).await;

    let session_token = jar
        .get("session_token")
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    let recent_listings = match state.listing_repo.find_all_paginated(0, 12, None).await {
        Ok((listings, _)) => listings
            .iter()
            .map(ListingSummaryDto::from_listing)
            .collect(),
        Err(e) => {
            tracing::error!("Error fetching recent listings from DB: {}", e);
            vec![]
        }
    };

    let template = HomeTemplate {
        current_user, 
        flash_success: None,
        flash_error: None,
        recent_listings,
        query_param: None,
        session_token,
    };
    Html(template.render().unwrap())
}
