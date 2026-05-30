use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::Json;
use common::errors::AppError;

use crate::adapters::payment_repository::PostgresPaymentRepository;
use crate::adapters::stripe_adapter::StripeAdapter;
use crate::dtos::WebhookResponse;
use crate::errors::PaymentError;
use crate::usecases::handle_webhook_usecase;

/// POST /payments/webhook
///
/// Receives Stripe webhook events.
/// - Reads body as raw Bytes BEFORE any parsing (critical for signature verification).
/// - Verifies the Stripe-Signature HMAC before processing.
/// - Does NOT require JWT authentication (uses Stripe signature instead).
pub async fn handle_webhook(
    headers: HeaderMap,
    State(stripe_adapter): State<StripeAdapter>,
    State(payment_repo): State<PostgresPaymentRepository>,
    body: Bytes,
) -> Result<(StatusCode, Json<WebhookResponse>), AppError> {
    // Extract the Stripe-Signature header
    let signature = headers
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            tracing::error!("Webhook recibido sin cabecera Stripe-Signature");
            AppError::BadRequest("Cabecera Stripe-Signature requerida".to_string())
        })?;

    // Process the webhook event
    handle_webhook_usecase::handle_webhook_usecase(
        &body,
        signature,
        &stripe_adapter,
        &payment_repo,
    )
    .await
    .map_err(|err| match err {
        PaymentError::InvalidSignature => {
            AppError::BadRequest("Firma de webhook inválida".to_string())
        }
        PaymentError::DatabaseError(e) => {
            tracing::error!("Error de base de datos en webhook: {:?}", e);
            AppError::Internal("Error interno del servidor".to_string())
        }
        _ => AppError::Internal("Error procesando webhook".to_string()),
    })?;

    Ok((StatusCode::OK, Json(WebhookResponse { received: true })))
}
