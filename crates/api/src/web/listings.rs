use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::{State, Query}, response::Html};
use axum_extra::extract::CookieJar;
use crate::app_state::AppState;
use users::dtos::UserDto;
use listings::dtos::ListingSummaryDto;
use listings::ports::ListingRepository;
use crate::web::filters;
use serde::Deserialize;
use common::auth::AuthUser;

#[derive(Deserialize)]
pub struct ListingsQuery {
    pub category: Option<String>,
    pub page: Option<i64>,
}

#[derive(Template)]
#[template(path = "pages/listings.html")]
pub struct ListingsTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub listings: Vec<ListingSummaryDto>,
    pub total_items: usize,
    pub current_page: usize,
    pub total_pages: usize,
    pub query_param: Option<String>,
    pub session_token: String,
}

pub async fn listings_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    jar: CookieJar,
    Query(query): Query<ListingsQuery>,
) -> impl IntoResponse {
    let current_user = crate::web::get_current_user(auth, &state).await;
    let page = query.page.unwrap_or(1);
    let page_index = if page > 0 { page - 1 } else { 0 };
    let per_page = 12;

    let session_token = jar
        .get("session_token")
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    let category_filter = query.category.as_deref().filter(|s| !s.is_empty());

    let (listings_dto, total_items) = match state.listing_repo.find_all_paginated(page_index, per_page, category_filter).await {
        Ok((listings, total)) => {
            let dtos = listings
                .iter()
                .map(ListingSummaryDto::from_listing)
                .collect();
            (dtos, total)
        }
        Err(e) => {
            tracing::error!("Error fetching listings from DB: {}", e);
            (vec![], 0)
        }
    };

    let total_pages = if total_items == 0 {
        1
    } else {
        ((total_items as f64) / (per_page as f64)).ceil() as usize
    };

    let template = ListingsTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        listings: listings_dto,
        total_items: total_items as usize,
        current_page: page as usize,
        total_pages,
        query_param: query.category,
        session_token,
    };
    Html(template.render().unwrap())
}
