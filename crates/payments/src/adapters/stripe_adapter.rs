use async_trait::async_trait;
use uuid::Uuid;

use crate::errors::PaymentError;
use crate::ports::StripePort;

/// Adapter for the Stripe API using the `async-stripe` SDK.
///
/// Handles PaymentIntent creation and webhook signature verification
/// via the official Stripe Rust library.
#[derive(Clone)]
pub struct StripeAdapter {
    client: stripe::Client,
    webhook_secret: String,
}

impl StripeAdapter {
    pub fn new(secret_key: String, webhook_secret: String) -> Self {
        let client = stripe::Client::new(&secret_key);
        Self {
            client,
            webhook_secret,
        }
    }
}

#[async_trait]
impl StripePort for StripeAdapter {
    async fn create_payment_intent(
        &self,
        amount_cents: i64,
        currency: &str,
        listing_id: Uuid,
        buyer_id: Uuid,
        idempotency_key: Uuid,
    ) -> Result<(String, String), PaymentError> {
        // Build metadata for traceability
        let mut metadata: stripe::Metadata = std::collections::HashMap::new();
        metadata.insert("listing_id".to_string(), listing_id.to_string());
        metadata.insert("buyer_id".to_string(), buyer_id.to_string());

        // Parse currency string to stripe::Currency
        let stripe_currency: stripe::Currency = serde_json::from_value(
            serde_json::Value::String(currency.to_lowercase()),
        )
        .map_err(|e| {
            PaymentError::StripeError(format!("Moneda no soportada '{}': {}", currency, e))
        })?;

        // Build the CreatePaymentIntent parameters
        let mut params = stripe::CreatePaymentIntent::new(amount_cents, stripe_currency);
        params.metadata = Some(metadata);
        params.automatic_payment_methods = Some(
            stripe::CreatePaymentIntentAutomaticPaymentMethods {
                enabled: true,
                ..Default::default()
            },
        );

        // Clone the client with an idempotent strategy to prevent duplicate charges
        // on network retries. The idempotency key is the payment_id UUID.
        let idempotent_client = self
            .client
            .clone()
            .with_strategy(stripe::RequestStrategy::Idempotent(
                idempotency_key.to_string(),
            ));

        // Create the PaymentIntent via the Stripe API
        let payment_intent = stripe::PaymentIntent::create(
            &idempotent_client,
            params,
        )
        .await
        .map_err(|e| {
            PaymentError::StripeError(format!("Error al crear PaymentIntent: {}", e))
        })?;

        let intent_id = payment_intent.id.to_string();
        let client_secret = payment_intent.client_secret.unwrap_or_default();

        Ok((intent_id, client_secret))
    }

    fn verify_webhook_signature(
        &self,
        payload: &[u8],
        signature_header: &str,
    ) -> Result<String, PaymentError> {
        // Convert payload bytes to string for Stripe SDK
        let payload_str = std::str::from_utf8(payload)
            .map_err(|_| PaymentError::InvalidSignature)?;

        // Use Stripe SDK's built-in webhook verification
        let event = stripe::Webhook::construct_event(
            payload_str,
            signature_header,
            &self.webhook_secret,
        )
        .map_err(|e| {
            tracing::error!("Error verificando firma de webhook de Stripe: {:?}", e);
            PaymentError::InvalidSignature
        })?;

        // Event type implements Display to return the string representation
        Ok(event.type_.to_string())
    }
}
