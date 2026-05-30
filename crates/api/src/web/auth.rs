use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Query, State},
    response::{Html, Redirect},
    Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use validator::Validate;

use crate::app_state::AppState;
use crate::filters;
use users::dtos::UserDto;
use users::dtos::{LoginDto, RegisterDto};

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

#[derive(Deserialize)]
pub struct AuthQuery {
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginFormData {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterFormData {
    pub email: String,
    pub password: String,
    pub display_name: String,
}

pub async fn login_handler(
    State(_state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> impl IntoResponse {
    let error_msg = if query.error.as_deref() == Some("1") {
        Some("Credenciales incorrectas. Por favor, inténtalo de nuevo.".to_string())
    } else {
        None
    };

    let template = LoginTemplate {
        current_user: None,
        flash_success: None,
        flash_error: error_msg,
        query_param: query.error,
    };
    Html(template.render().unwrap())
}

pub async fn register_handler(
    State(_state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> impl IntoResponse {
    let error_msg = if query.error.as_deref() == Some("1") {
        Some("Error al registrar la cuenta. El correo podría estar ya en uso o los datos son inválidos.".to_string())
    } else {
        None
    };

    let template = RegisterTemplate {
        current_user: None,
        flash_success: None,
        flash_error: error_msg,
        query_param: query.error,
    };
    Html(template.render().unwrap())
}

pub async fn login_post_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<LoginFormData>,
) -> impl IntoResponse {
    let dto = LoginDto {
        email: form.email,
        password: form.password,
    };

    if dto.validate().is_err() {
        return Redirect::to("/login?error=1").into_response();
    }

    match users::usecases::login_usecase::login(&state.user_repo, dto, &state.jwt_secret).await {
        Ok(res) => {
            let mut cookie = Cookie::new("session_token", res.access_token);
            cookie.set_path("/");
            cookie.set_http_only(true);
            cookie.set_same_site(SameSite::Lax);
            cookie.set_max_age(Some(cookie::time::Duration::seconds(86400)));

            let jar = jar.add(cookie);
            (jar, Redirect::to("/")).into_response()
        }
        Err(_) => {
            Redirect::to("/login?error=1").into_response()
        }
    }
}

pub async fn register_post_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<RegisterFormData>,
) -> impl IntoResponse {
    let dto = RegisterDto {
        email: form.email,
        password: form.password,
        display_name: form.display_name,
    };

    if dto.validate().is_err() {
        return Redirect::to("/register?error=1").into_response();
    }

    match users::usecases::register_usecase::register(&state.user_repo, dto, &state.jwt_secret).await {
        Ok(res) => {
            let mut cookie = Cookie::new("session_token", res.access_token);
            cookie.set_path("/");
            cookie.set_http_only(true);
            cookie.set_same_site(SameSite::Lax);
            cookie.set_max_age(Some(cookie::time::Duration::seconds(86400)));

            let jar = jar.add(cookie);
            (jar, Redirect::to("/")).into_response()
        }
        Err(_) => {
            Redirect::to("/register?error=1").into_response()
        }
    }
}

pub async fn logout_handler(
    jar: CookieJar,
) -> impl IntoResponse {
    let mut cookie = Cookie::new("session_token", "");
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(Some(cookie::time::Duration::seconds(0)));

    let jar = jar.add(cookie);
    (jar, Redirect::to("/"))
}
