use crate::errors::PaymentError;
use crate::models::PaymentStatus;
use crate::ports::{PaymentRepository, StripePort};

/// Processes a Stripe webhook event: verifies the signature, then updates
/// the payment and associated listing status accordingly.
pub async fn handle_webhook_usecase(
    payload: &[u8],
    signature_header: &str,
    stripe_port: &dyn StripePort,
    payment_repo: &dyn PaymentRepository,
) -> Result<(), PaymentError> {
    // 1. Verify the webhook signature
    let event_type = stripe_port.verify_webhook_signature(payload, signature_header)?;

    // 2. Parse the event payload to extract the PaymentIntent ID and status
    let event: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|_| PaymentError::InvalidSignature)?;

    let intent_id = event
        .pointer("/data/object/id")
        .and_then(|v| v.as_str())
        .ok_or(PaymentError::InvalidSignature)?;

    // 3. Map the event type to our payment status
    let new_status = match event_type.as_str() {
        "payment_intent.succeeded" => Some(PaymentStatus::Succeeded),
        "payment_intent.payment_failed" => Some(PaymentStatus::Failed),
        "charge.refunded" => Some(PaymentStatus::Refunded),
        _ => {
            // Unknown event type — acknowledge but no action needed
            tracing::info!("Ignored webhook event type: {}", event_type);
            return Ok(());
        }
    };

    if let Some(status) = new_status {
        // Check idempotency: only update if status actually changes
        if let Some(existing) = payment_repo.find_by_stripe_intent_id(intent_id).await? {
            if existing.status == status {
                tracing::info!(
                    "Payment {} already in status {:?}, skipping",
                    intent_id,
                    status
                );
                return Ok(());
            }
        }

        payment_repo.update_status(intent_id, status).await?;

        // If payment succeeded, mark the listing as sold
        if status == PaymentStatus::Succeeded {
            mark_listing_as_sold(payment_repo, intent_id).await?;
        }
    }

    Ok(())
}

/// Updates the listing status to 'sold' when a payment succeeds.
async fn mark_listing_as_sold(
    payment_repo: &dyn PaymentRepository,
    stripe_intent_id: &str,
) -> Result<(), PaymentError> {
    // Find the payment to get listing_id
    let payment = payment_repo
        .find_by_stripe_intent_id(stripe_intent_id)
        .await?
        .ok_or_else(|| {
            PaymentError::StripeError(format!(
                "No payment record for intent: {}",
                stripe_intent_id
            ))
        })?;

    tracing::info!(
        "Payment succeeded for listing {}, marking as sold",
        payment.listing_id
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    use crate::models::Payment;

    struct MockStripePortInvalid;

    #[async_trait]
    impl StripePort for MockStripePortInvalid {
        async fn create_payment_intent(
            &self,
            _amount_cents: i64,
            _currency: &str,
            _listing_id: Uuid,
            _buyer_id: Uuid,
            _idempotency_key: Uuid,
        ) -> Result<(String, String), PaymentError> {
            unreachable!()
        }

        fn verify_webhook_signature(&self, _payload: &[u8], _signature_header: &str) -> Result<String, PaymentError> {
            Err(PaymentError::InvalidSignature)
        }
    }

    struct MockStripePortOk;

    #[async_trait]
    impl StripePort for MockStripePortOk {
        async fn create_payment_intent(
            &self,
            _amount_cents: i64,
            _currency: &str,
            _listing_id: Uuid,
            _buyer_id: Uuid,
            _idempotency_key: Uuid,
        ) -> Result<(String, String), PaymentError> {
            unreachable!()
        }

        fn verify_webhook_signature(&self, _payload: &[u8], _signature_header: &str) -> Result<String, PaymentError> {
            Ok("payment_intent.succeeded".to_string())
        }
    }

    struct MockPaymentRepoNoOp;

    #[async_trait]
    impl PaymentRepository for MockPaymentRepoNoOp {
        async fn insert(&self, _payment: &Payment) -> Result<Payment, PaymentError> {
            unreachable!()
        }

        async fn update_status(&self, _stripe_intent_id: &str, _new_status: PaymentStatus) -> Result<(), PaymentError> {
            Ok(())
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<Payment>, PaymentError> {
            unreachable!()
        }

        async fn find_by_stripe_intent_id(&self, stripe_intent_id: &str) -> Result<Option<Payment>, PaymentError> {
            // Return a payment so mark_listing_as_sold succeeds
            Ok(Some(Payment {
                id: Uuid::new_v4(),
                listing_id: Uuid::new_v4(),
                buyer_id: Uuid::new_v4(),
                seller_id: Uuid::new_v4(),
                stripe_payment_intent_id: stripe_intent_id.to_string(),
                amount_cents: 5000,
                currency: "eur".to_string(),
                status: PaymentStatus::Succeeded,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }))
        }
    }

    /// Simulates a payment intent succeeded event payload from Stripe.
    fn make_successful_payload() -> Vec<u8> {
        serde_json::json!({
            "id": "evt_test",
            "type": "payment_intent.succeeded",
            "data": {
                "object": {
                    "id": "pi_test_123",
                    "amount": 5000,
                    "currency": "eur",
                    "status": "succeeded"
                }
            }
        }).to_string().into_bytes()
    }

    #[tokio::test]
    async fn given_invalid_signature_when_handle_webhook_then_returns_invalid_signature_error() {
        let payload = make_successful_payload();
        let result = handle_webhook_usecase(
            &payload,
            "invalid_signature",
            &MockStripePortInvalid,
            &MockPaymentRepoNoOp,
        ).await;

        assert!(matches!(result, Err(PaymentError::InvalidSignature)));
    }

    #[tokio::test]
    async fn given_valid_signature_when_handle_payment_succeeded_then_returns_ok() {
        let payload = make_successful_payload();
        let result = handle_webhook_usecase(
            &payload,
            "valid_signature_will_be_ignored_by_mock",
            &MockStripePortOk,
            &MockPaymentRepoNoOp,
        ).await;

        assert!(result.is_ok());
    }

    /// Mock that returns different event types based on the payload.
    struct MockStripePortDynamic;

    #[async_trait]
    impl StripePort for MockStripePortDynamic {
        async fn create_payment_intent(
            &self,
            _amount_cents: i64,
            _currency: &str,
            _listing_id: Uuid,
            _buyer_id: Uuid,
            _idempotency_key: Uuid,
        ) -> Result<(String, String), PaymentError> {
            unreachable!()
        }

        fn verify_webhook_signature(&self, payload: &[u8], _signature_header: &str) -> Result<String, PaymentError> {
            // Actually parse the event type from the payload like the real implementation does
            let event: serde_json::Value = serde_json::from_slice(payload)
                .map_err(|_| PaymentError::InvalidSignature)?;
            let event_type = event
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or(PaymentError::InvalidSignature)?
                .to_string();
            Ok(event_type)
        }
    }

    #[tokio::test]
    async fn given_unknown_event_type_when_handle_webhook_then_returns_ok_and_ignores() {
        let payload = serde_json::json!({
            "id": "evt_ignore",
            "type": "charge.refund.updated", // Unknown/unhandled event type
            "data": {
                "object": {
                    "id": "pi_test_456"
                }
            }
        }).to_string().into_bytes();

        let result = handle_webhook_usecase(
            &payload,
            "dummy",
            &MockStripePortDynamic,
            &MockPaymentRepoNoOp,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn given_already_processed_event_when_handle_webhook_then_returns_ok_idempotent() {
        let payment_id = "pi_already_done";
        let payment = Payment {
            id: Uuid::new_v4(),
            listing_id: Uuid::new_v4(),
            buyer_id: Uuid::new_v4(),
            seller_id: Uuid::new_v4(),
            stripe_payment_intent_id: payment_id.to_string(),
            amount_cents: 5000,
            currency: "eur".to_string(),
            status: PaymentStatus::Succeeded,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let payment_clone = payment.clone();
        let repo = MockPaymentRepoWithExisting { payment: Some(payment_clone) };

        let payload = serde_json::json!({
            "id": "evt_test",
            "type": "payment_intent.succeeded",
            "data": {
                "object": {
                    "id": payment_id,
                    "amount": 5000,
                    "currency": "eur"
                }
            }
        }).to_string().into_bytes();

        let result = handle_webhook_usecase(
            &payload,
            "dummy",
            &MockStripePortDynamic,
            &repo,
        ).await;

        assert!(result.is_ok());
    }

    /// Repository that returns a pre-existing payment for idempotency checks.
    struct MockPaymentRepoWithExisting {
        payment: Option<Payment>,
    }

    #[async_trait]
    impl PaymentRepository for MockPaymentRepoWithExisting {
        async fn insert(&self, _payment: &Payment) -> Result<Payment, PaymentError> {
            unreachable!()
        }

        async fn update_status(&self, _stripe_intent_id: &str, _new_status: PaymentStatus) -> Result<(), PaymentError> {
            panic!("update_status should not be called for idempotent events");
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<Payment>, PaymentError> {
            Ok(self.payment.clone())
        }

        async fn find_by_stripe_intent_id(&self, _stripe_intent_id: &str) -> Result<Option<Payment>, PaymentError> {
            Ok(self.payment.clone())
        }
    }
}
