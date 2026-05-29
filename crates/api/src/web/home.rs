use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::State, response::Html};
use crate::app_state::AppState;
use users::dtos::UserDto;
use listings::dtos::ListingSummaryDto;

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct HomeTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub recent_listings: Vec<ListingSummaryDto>,
    pub query_param: Option<String>,
}

pub async fn home_handler(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let template = HomeTemplate {
        current_user: None, 
        flash_success: None,
        flash_error: None,
        recent_listings: vec![],
        query_param: None,
    };
    Html(template.render().unwrap())
}
