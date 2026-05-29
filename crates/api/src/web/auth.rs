use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::State, response::Html};
use crate::app_state::AppState;
use users::dtos::UserDto;
use crate::web::filters;

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub query_param: Option<String>,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub query_param: Option<String>,
}

pub async fn login_handler(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let template = LoginTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        query_param: None,
    };
    Html(template.render().unwrap())
}

pub async fn register_handler(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let template = RegisterTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        query_param: None,
    };
    Html(template.render().unwrap())
}
