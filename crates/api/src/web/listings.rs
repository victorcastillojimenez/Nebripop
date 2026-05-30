use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::{State, Query}, response::Html};
use axum_extra::extract::CookieJar;
use rust_decimal::Decimal;
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
    #[serde(default)]
    pub condition: Option<Vec<String>>,
    pub min_price: Option<String>,
    pub max_price: Option<String>,
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
    /// The currently selected category (to highlight in the sidebar).
    pub selected_category: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    /// The currently selected condition values (checkbox state).
    pub selected_conditions: Vec<String>,
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
    // Take the first condition value from the list (supports multiple checkboxes,
    // though the repository accepts a single condition for now).
    let condition_value = query.condition.as_ref()
        .and_then(|v| v.first().map(|s| s.as_str()))
        .filter(|s| !s.is_empty());
    // Parse optional price range from string parameters.
    let min_price = query.min_price.as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|&v| v > 0.0)
        .map(|v| Decimal::from_f64_retain(v).unwrap_or_default());
    let max_price = query.max_price.as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|&v| v > 0.0)
        .map(|v| Decimal::from_f64_retain(v).unwrap_or_default());

    let (listings_dto, total_items) = match state.listing_repo.find_all_paginated(
        page_index,
        per_page,
        category_filter,
        condition_value,
        min_price,
        max_price,
    ).await {
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

    // Convert price strings to f64 for template display
    let min_price_display = query.min_price.as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|&v| v > 0.0);
    let max_price_display = query.max_price.as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|&v| v > 0.0);

    let selected_conditions = query.condition.unwrap_or_default();

    let template = ListingsTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        listings: listings_dto,
        total_items: total_items as usize,
        current_page: page as usize,
        total_pages,
        query_param: query.category.clone(),
        session_token,
        selected_category: query.category,
        min_price: min_price_display,
        max_price: max_price_display,
        selected_conditions,
    };
    Html(template.render().unwrap())
}
