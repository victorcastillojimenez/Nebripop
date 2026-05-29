use askama::Template;
use axum::{extract::{State, Query}, response::Html, http::StatusCode};
use crate::app_state::AppState;
use users::dtos::UserDto;
use listings::dtos::ListingSummaryDto;
use listings::models::{ListingStatus, PhysicalCondition};
use search::dtos::SearchQueryDto;
use search::ports::SearchEngine;
use serde::Deserialize;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{TimeZone, Utc};
use crate::web::filters;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub condition: Option<Vec<String>>,
    pub page: Option<i64>,
}

#[derive(Template)]
#[template(path = "pages/search_results.html")]
pub struct SearchResultsTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub listings: Vec<ListingSummaryDto>,
    pub query_param: Option<String>,
    pub total_items: usize,
    pub current_page: usize,
    pub total_pages: usize,
    pub selected_category: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub selected_conditions: Vec<String>,
}

pub async fn search_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Html<String>, StatusCode> {
    let page = params.page.unwrap_or(1);
    let page_index = if page > 0 { page - 1 } else { 0 };
    let per_page = 12;

    let query_dto = SearchQueryDto {
        query: params.q.clone().filter(|s| !s.is_empty()),
        category: params.category.clone().filter(|s| !s.is_empty()),
        min_price: params.min_price,
        max_price: params.max_price,
        latitude: None,
        longitude: None,
        radius_km: None,
        sort: None,
        page: page_index,
        per_page,
    };

    let engine = state.search_engine.as_ref().map(|e| e as &dyn SearchEngine);
    let (search_res, _engine_used) = search::usecases::search_usecase::execute(
        engine,
        &state.pool,
        query_dto,
    ).await.map_err(|e| {
        tracing::error!("Error executing search: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let listings_dto: Vec<ListingSummaryDto> = search_res.items.into_iter().map(|item| {
        let condition = match item.condition.to_lowercase().as_str() {
            "new" => PhysicalCondition::New,
            "like_new" | "like new" => PhysicalCondition::LikeNew,
            _ => PhysicalCondition::Used,
        };

        let has_image = item.image_url.is_some();

        ListingSummaryDto {
            id: item.id,
            seller_id: Uuid::nil(),
            title: item.title,
            price: Decimal::from_f64_retain(item.price).unwrap_or_default(),
            currency: item.currency,
            category: item.category,
            condition,
            status: ListingStatus::Active,
            city: item.city,
            first_image_url: item.image_url,
            image_count: if has_image { 1 } else { 0 },
            created_at: Utc.timestamp_opt(item.created_at, 0).single().unwrap_or_else(Utc::now),
            updated_at: Utc.timestamp_opt(item.created_at, 0).single().unwrap_or_else(Utc::now),
        }
    }).collect();

    let total_pages = if search_res.total == 0 {
        1
    } else {
        ((search_res.total as f64) / (per_page as f64)).ceil() as usize
    };

    let template = SearchResultsTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        listings: listings_dto,
        query_param: params.q,
        total_items: search_res.total as usize,
        current_page: page as usize,
        total_pages,
        selected_category: params.category,
        min_price: params.min_price,
        max_price: params.max_price,
        selected_conditions: params.condition.unwrap_or_default(),
    };

    template.render()
        .map(Html)
        .map_err(|e| {
            tracing::error!("Failed to render search results template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
