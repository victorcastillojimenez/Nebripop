use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use common::auth::AuthUser;
use common::errors::AppError;

use crate::adapters::listing_adapter::PaymentsListingAdapter;
use crate::adapters::payment_repository::PostgresPaymentRepository;
use crate::adapters::stripe_adapter::StripeAdapter;
use crate::dtos::{CreateIntentDto, CreateIntentResponse};
use crate::errors::PaymentError;
use crate::usecases::create_intent_usecase;

/// POST /payments/intent
///
/// Creates a Stripe PaymentIntent for a given listing.
/// Requires JWT authentication (AuthUser extractor).
pub async fn handle_create_intent(
    auth_user: AuthUser,
    State(payment_repo): State<PostgresPaymentRepository>,
    State(stripe_adapter): State<StripeAdapter>,
    State(listing_service): State<PaymentsListingAdapter>,
    Json(dto): Json<CreateIntentDto>,
) -> Result<(StatusCode, Json<CreateIntentResponse>), AppError> {
    let response = create_intent_usecase::create_intent_usecase(
        dto,
        auth_user.id,
        &listing_service,
        &payment_repo,
        &stripe_adapter,
    )
    .await
    .map_err(map_payment_error)?;

    Ok((StatusCode::CREATED, Json(response)))
}

/// Maps PaymentError to AppError for consistent HTTP responses.
fn map_payment_error(err: PaymentError) -> AppError {
    match err {
        PaymentError::NotFound(id) => {
            AppError::NotFound(format!("Pago con ID {} no encontrado", id))
        }
        PaymentError::Forbidden(_) => {
            AppError::Forbidden("No tienes permiso para acceder a este pago".to_string())
        }
        PaymentError::SelfPurchase => {
            AppError::BadRequest("No puedes comprar tu propio anuncio".to_string())
        }
        PaymentError::ListingNotAvailable(_) => {
            AppError::BadRequest("El anuncio no está disponible para la compra".to_string())
        }
        PaymentError::InvalidSignature => {
            AppError::BadRequest("Firma de webhook inválida".to_string())
        }
        PaymentError::StripeError(msg) => {
            AppError::BadRequest(format!("Error de Stripe: {}", msg))
        }
        PaymentError::ValidationError(msg) => AppError::ValidationError(msg),
        PaymentError::DatabaseError(e) => {
            tracing::error!("Error de base de datos en pagos: {:?}", e);
            AppError::Internal("Error interno del servidor".to_string())
        }
    }
}
