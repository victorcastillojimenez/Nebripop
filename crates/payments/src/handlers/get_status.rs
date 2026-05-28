use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use common::auth::AuthUser;
use common::errors::AppError;
use uuid::Uuid;

use crate::adapters::payment_repository::PostgresPaymentRepository;
use crate::dtos::PaymentStatusDto;
use crate::errors::PaymentError;
use crate::usecases::get_payment_status_usecase;

/// GET /payments/:id/status
///
/// Returns the status of a payment.
/// Only the buyer or the seller of the payment can access it.
pub async fn handle_get_status(
    auth_user: AuthUser,
    State(payment_repo): State<PostgresPaymentRepository>,
    Path(payment_id): Path<Uuid>,
) -> Result<(StatusCode, Json<PaymentStatusDto>), AppError> {
    let dto = get_payment_status_usecase::get_payment_status_usecase(
        payment_id,
        auth_user.id,
        &payment_repo,
    )
    .await
    .map_err(|err| match err {
        PaymentError::NotFound(id) => {
            AppError::NotFound(format!("Pago con ID {} no encontrado", id))
        }
        PaymentError::Forbidden(_) => {
            AppError::Forbidden("No tienes permiso para acceder a este pago".to_string())
        }
        PaymentError::DatabaseError(e) => {
            tracing::error!("Error de base de datos: {:?}", e);
            AppError::Internal("Error interno del servidor".to_string())
        }
        _ => AppError::Internal("Error interno del servidor".to_string()),
    })?;

    Ok((StatusCode::OK, Json(dto)))
}
