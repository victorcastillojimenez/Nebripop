use uuid::Uuid;

use crate::dtos::PaymentStatusDto;
use crate::errors::PaymentError;
use crate::ports::PaymentRepository;

/// Retrieves the status of a payment, verifying that the requesting user
/// is either the buyer or the seller of the payment.
pub async fn get_payment_status_usecase(
    payment_id: Uuid,
    requesting_user_id: Uuid,
    payment_repo: &dyn PaymentRepository,
) -> Result<PaymentStatusDto, PaymentError> {
    let payment = payment_repo
        .find_by_id(payment_id)
        .await?
        .ok_or(PaymentError::NotFound(payment_id))?;

    // Authorization: only buyer or seller can view the status
    if payment.buyer_id != requesting_user_id && payment.seller_id != requesting_user_id {
        return Err(PaymentError::Forbidden(requesting_user_id));
    }

    Ok(PaymentStatusDto {
        id: payment.id,
        status: payment.status.as_db_string().to_string(),
        amount_cents: payment.amount_cents,
        currency: payment.currency,
        created_at: payment.created_at,
    })
}
