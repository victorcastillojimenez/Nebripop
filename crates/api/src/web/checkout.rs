use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::State, response::Html};
use crate::app_state::AppState;
use users::dtos::UserDto;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "payments/checkout.html")]
pub struct CheckoutTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub listing_id: Uuid,
    pub listing_title: String,
    pub listing_image: Option<String>,
    pub listing_condition: Option<String>,
    pub seller_name: String,
    pub price: f64,
    pub query_param: Option<String>,
}

#[derive(Template)]
#[template(path = "payments/success.html")]
pub struct PaymentSuccessTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub listing_title: String,
    pub seller_name: String,
    pub amount: f64,
    pub payment_id: Option<String>,
    pub conversation_id: Option<Uuid>,
    pub query_param: Option<String>,
}

#[derive(Template)]
#[template(path = "payments/error.html")]
pub struct PaymentErrorTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub error_message: Option<String>,
    pub listing_id: Option<Uuid>,
    pub query_param: Option<String>,
}

pub async fn checkout_handler(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let template = CheckoutTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        listing_id: Uuid::new_v4(),
        listing_title: "Producto".to_string(),
        listing_image: None,
        listing_condition: None,
        seller_name: "Vendedor".to_string(),
        price: 0.0,
        query_param: None,
    };
    Html(template.render().unwrap())
}

pub async fn payment_success_handler(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let template = PaymentSuccessTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        listing_title: "".to_string(),
        seller_name: "".to_string(),
        amount: 0.0,
        payment_id: None,
        conversation_id: None,
        query_param: None,
    };
    Html(template.render().unwrap())
}

pub async fn payment_error_handler(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let template = PaymentErrorTemplate {
        current_user: None,
        flash_success: None,
        flash_error: None,
        error_message: Some("Ha ocurrido un error inesperado.".to_string()),
        listing_id: None,
        query_param: None,
    };
    Html(template.render().unwrap())
}
