use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::State, response::Html};
use crate::app_state::AppState;
use users::dtos::UserDto;
use crate::web::filters;

#[derive(Template)]
#[template(path = "pages/listing_create.html")]
pub struct ListingCreateTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub query_param: Option<String>,
}

pub async fn listing_create_handler(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let template = ListingCreateTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        query_param: None,
    };
    Html(template.render().unwrap())
}
