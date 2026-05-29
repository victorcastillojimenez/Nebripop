use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::{State, Path, Query}, response::{Html, Redirect}};
use crate::app_state::AppState;
use users::dtos::UserDto;
use users::ports::UserRepositoryPort;
use listings::ports::ListingRepository;
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;
use crate::web::filters;
use common::auth::AuthUser;

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
    pub client_secret: String,
    pub stripe_publishable_key: String,
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

#[derive(serde::Deserialize)]
pub struct PaymentSuccessQuery {
    pub listing_title: Option<String>,
    pub seller_name: Option<String>,
    pub amount: Option<f64>,
    pub payment_id: Option<String>,
    pub conversation_id: Option<Uuid>,
}

#[derive(serde::Deserialize)]
pub struct PaymentErrorQuery {
    pub error_message: Option<String>,
    pub listing_id: Option<Uuid>,
}

pub async fn checkout_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let auth_user = match auth {
        Some(user) => user,
        None => return Redirect::to("/login").into_response(),
    };

    let current_user = crate::web::get_current_user(Some(auth_user.clone()), &state).await;

    // 1. Obtener el listing real por ID
    let listing = match state.listing_repo.find_by_id(id).await {
        Ok(Some(l)) => l,
        Ok(None) => {
            return Redirect::to("/payments/error?error_message=El+anuncio+no+existe").into_response();
        }
        Err(e) => {
            tracing::error!("Error fetching listing for checkout: {:?}", e);
            return Redirect::to("/payments/error?error_message=Error+al+buscar+el+anuncio").into_response();
        }
    };

    // 2. Obtener el vendedor
    let seller = match state.user_repo.find_by_id(listing.seller_id).await {
        Ok(Some(s)) => s,
        _ => {
            return Redirect::to("/payments/error?error_message=Vendedor+no+encontrado").into_response();
        }
    };

    // 3. Crear el PaymentIntent via create_intent_usecase
    let dto = payments::dtos::CreateIntentDto {
        listing_id: id,
        currency: "eur".to_string(),
    };

    let intent_res = payments::usecases::create_intent_usecase::create_intent_usecase(
        dto,
        auth_user.id,
        &state.listing_service,
        &state.payment_repo,
        &state.stripe_adapter,
    ).await;

    let create_intent_response = match intent_res {
        Ok(res) => res,
        Err(err) => {
            let msg = match err {
                payments::errors::PaymentError::SelfPurchase => {
                    "No puedes comprar tu propio anuncio.".to_string()
                }
                payments::errors::PaymentError::ListingNotAvailable(_) => {
                    "El anuncio no está disponible para la compra.".to_string()
                }
                payments::errors::PaymentError::StripeError(msg) => {
                    format!("Error de Stripe: {}", msg)
                }
                _ => "No se pudo procesar la solicitud de pago.".to_string(),
            };
            let encoded_msg = msg.replace(' ', "+");
            return Redirect::to(&format!(
                "/payments/error?error_message={}&listing_id={}",
                encoded_msg,
                id
            )).into_response();
        }
    };

    let price_f64 = listing.price.to_f64().unwrap_or(0.0);
    let listing_image = listing.images.first().map(|img| img.image_url.clone());
    let listing_condition = Some(listing.condition.to_string());

    let template = CheckoutTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        listing_id: id,
        listing_title: listing.title,
        listing_image,
        listing_condition,
        seller_name: seller.display_name,
        price: price_f64,
        query_param: None,
        client_secret: create_intent_response.client_secret,
        stripe_publishable_key: state.stripe_publishable_key.clone(),
    };

    Html(template.render().unwrap()).into_response()
}

pub async fn payment_success_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    Query(query): Query<PaymentSuccessQuery>,
) -> impl IntoResponse {
    let current_user = crate::web::get_current_user(auth, &state).await;
    let template = PaymentSuccessTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        listing_title: query.listing_title.unwrap_or_else(|| "Artículo".to_string()),
        seller_name: query.seller_name.unwrap_or_else(|| "Vendedor".to_string()),
        amount: query.amount.unwrap_or(0.0),
        payment_id: query.payment_id,
        conversation_id: query.conversation_id,
        query_param: None,
    };
    Html(template.render().unwrap())
}

pub async fn payment_error_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    Query(query): Query<PaymentErrorQuery>,
) -> impl IntoResponse {
    let current_user = crate::web::get_current_user(auth, &state).await;
    let template = PaymentErrorTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        error_message: query.error_message.or_else(|| Some("Ha ocurrido un error inesperado.".to_string())),
        listing_id: query.listing_id,
        query_param: None,
    };
    Html(template.render().unwrap())
}
