use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::State, response::Html};
use crate::app_state::AppState;
use users::dtos::UserDto;
use listings::dtos::ListingSummaryDto;
use crate::web::filters;

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
}

pub async fn listings_handler(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let template = ListingsTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        listings: vec![],
        total_items: 0,
        current_page: 1,
        total_pages: 1,
        query_param: None,
    };
    Html(template.render().unwrap())
}
