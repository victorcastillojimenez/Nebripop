use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::{State, Query, Form}, response::Html};
use axum_extra::extract::CookieJar;
use crate::app_state::AppState;
use users::dtos::UserDto;
use crate::web::filters;
use common::auth::AuthUser;
use listings::dtos::CreateListingDto;
use listings::usecases::create_listing_usecase::create_listing_usecase;
use search::ports::SearchEngine;

#[derive(serde::Deserialize)]
pub struct ListingCreateQuery {
    pub error: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct CreateListingForm {
    pub title: String,
    pub description: String,
    pub price: rust_decimal::Decimal,
    pub category: String,
    pub condition: listings::models::PhysicalCondition,
    pub city: String,
}

#[derive(Template)]
#[template(path = "pages/listing_create.html")]
pub struct ListingCreateTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub query_param: Option<String>,
    pub session_token: String,
}

pub async fn listing_create_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    jar: CookieJar,
    Query(query): Query<ListingCreateQuery>,
) -> impl IntoResponse {
    let auth_user = match auth {
        Some(au) => au,
        None => {
            return axum::response::Redirect::to("/login?next=/listings/create").into_response();
        }
    };

    let current_user = crate::web::get_current_user(Some(auth_user), &state).await;
    if current_user.is_none() {
        return axum::response::Redirect::to("/login?next=/listings/create").into_response();
    }

    let session_token = jar
        .get("session_token")
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    let flash_error = if query.error.is_some() {
        Some("Ha ocurrido un error al crear el anuncio. Por favor, comprueba los campos e inténtalo de nuevo.".to_string())
    } else {
        None
    };

    let template = ListingCreateTemplate {
        current_user,
        flash_success: None,
        flash_error,
        query_param: query.error,
        session_token,
    };
    Html(template.render().unwrap()).into_response()
}

pub async fn listing_create_post_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    Form(form): Form<CreateListingForm>,
) -> impl IntoResponse {
    let auth_user = match auth {
        Some(au) => au,
        None => {
            return axum::http::StatusCode::FORBIDDEN.into_response();
        }
    };

    let dto = CreateListingDto {
        title: form.title,
        description: form.description,
        price: form.price,
        category: form.category,
        condition: form.condition,
        location_lat: 40.416775, // Madrid default coords
        location_lon: -3.703790,
        city: form.city,
    };

    let search_engine_ref = state.search_engine.as_ref().map(|s| s as &dyn SearchEngine);

    match create_listing_usecase(&state.listing_repo, search_engine_ref, auth_user.id, dto).await {
        Ok(_) => {
            axum::response::Response::builder()
                .status(axum::http::StatusCode::SEE_OTHER)
                .header(axum::http::header::LOCATION, "/listings")
                .body(axum::body::Body::empty())
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Error creating listing: {:?}", e);
            axum::response::Response::builder()
                .status(axum::http::StatusCode::SEE_OTHER)
                .header(axum::http::header::LOCATION, "/listings/create?error=1")
                .body(axum::body::Body::empty())
                .unwrap()
        }
    }
}
