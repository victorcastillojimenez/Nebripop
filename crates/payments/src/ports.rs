use async_trait::async_trait;
use uuid::Uuid;

use crate::errors::PaymentError;
use crate::models::{Payment, PaymentStatus};

/// Repository port for payment persistence operations.
#[async_trait]
pub trait PaymentRepository: Send + Sync {
    /// Insert a new payment record.
    async fn insert(&self, payment: &Payment) -> Result<Payment, PaymentError>;

    /// Update the status of a payment by its Stripe PaymentIntent ID.
    async fn update_status(
        &self,
        stripe_intent_id: &str,
        new_status: PaymentStatus,
    ) -> Result<(), PaymentError>;

    /// Find a payment by its internal ID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Payment>, PaymentError>;

    /// Find a payment by its Stripe PaymentIntent ID.
    async fn find_by_stripe_intent_id(
        &self,
        stripe_intent_id: &str,
    ) -> Result<Option<Payment>, PaymentError>;
}

/// Port for interacting with the Stripe API.
#[async_trait]
pub trait StripePort: Send + Sync {
    /// Create a Stripe PaymentIntent and return (intent_id, client_secret).
    async fn create_payment_intent(
        &self,
        amount_cents: i64,
        currency: &str,
        listing_id: Uuid,
        buyer_id: Uuid,
    ) -> Result<(String, String), PaymentError>;

    /// Verify the HMAC-SHA256 signature of a Stripe webhook payload.
    /// Returns the raw event type string on success (e.g. "payment_intent.succeeded").
    fn verify_webhook_signature(
        &self,
        payload: &[u8],
        signature_header: &str,
    ) -> Result<String, PaymentError>;
}
