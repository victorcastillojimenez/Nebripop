use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::{State, Query}, response::Html};
use crate::app_state::AppState;
use users::dtos::UserDto;
use listings::dtos::ListingSummaryDto;
use serde::Deserialize;
use crate::web::filters;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
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
    State(_state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let template = SearchResultsTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        listings: vec![],
        query_param: params.q,
        total_items: 0,
        current_page: 1,
        total_pages: 1,
        selected_category: None,
        min_price: None,
        max_price: None,
        selected_conditions: vec![],
    };
    Html(template.render().unwrap())
}
