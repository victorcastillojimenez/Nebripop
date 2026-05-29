use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::errors::PaymentError;
use crate::ports::StripePort;

type HmacSha256 = Hmac<Sha256>;

/// Adapter for the Stripe API using direct HTTP calls.
#[derive(Clone)]
pub struct StripeAdapter {
    secret_key: String,
    webhook_secret: String,
    http_client: reqwest::Client,
}

impl StripeAdapter {
    pub fn new(secret_key: String, webhook_secret: String) -> Self {
        Self {
            secret_key,
            webhook_secret,
            http_client: reqwest::Client::new(),
        }
    }

    fn auth_header_value(&self) -> String {
        format!("Bearer {}", self.secret_key)
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
        let metadata = serde_json::json!({
            "listing_id": listing_id.to_string(),
            "buyer_id": buyer_id.to_string(),
        });

        let params = serde_json::json!({
            "amount": amount_cents,
            "currency": currency,
            "metadata": metadata,
            "automatic_payment_methods": {
                "enabled": true
            }
        });

        let response = self
            .http_client
            .post("https://api.stripe.com/v1/payment_intents")
            .header("Authorization", self.auth_header_value())
            .header("Content-Type", "application/json")
            .header("Idempotency-Key", idempotency_key.to_string())
            .json(&params)
            .send()
            .await
            .map_err(|e| PaymentError::StripeError(format!("Error de conexión con Stripe: {}", e)))?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| PaymentError::StripeError(format!("Error al parsear respuesta de Stripe: {}", e)))?;

        if !status.is_success() {
            let error_msg = body
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Error desconocido de Stripe");
            return Err(PaymentError::StripeError(error_msg.to_string()));
        }

        let intent_id = body
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PaymentError::StripeError("ID de PaymentIntent no encontrado en respuesta".to_string()))?
            .to_string();

        let client_secret = body
            .get("client_secret")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PaymentError::StripeError("client_secret no encontrado en respuesta".to_string()))?
            .to_string();

        Ok((intent_id, client_secret))
    }

    fn verify_webhook_signature(
        &self,
        payload: &[u8],
        signature_header: &str,
    ) -> Result<String, PaymentError> {
        // Parse the Stripe-Signature header
        let mut expected_signature: Option<String> = None;
        let mut expected_timestamp: Option<i64> = None;

        for part in signature_header.split(',') {
            let part = part.trim();
            if let Some(value) = part.strip_prefix("t=") {
                expected_timestamp = value.parse::<i64>().ok();
            } else if let Some(value) = part.strip_prefix("v1=") {
                expected_signature = Some(value.to_string());
            }
        }

        let signature = expected_signature
            .ok_or(PaymentError::InvalidSignature)?;

        let timestamp = expected_timestamp
            .ok_or(PaymentError::InvalidSignature)?;

        // Construct the signed payload string: timestamp + "." + payload
        let signed_payload = format!("{}.{}", timestamp, std::str::from_utf8(payload).unwrap_or_default());

        // Compute HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(self.webhook_secret.as_bytes())
            .map_err(|_| PaymentError::InvalidSignature)?;

        mac.update(signed_payload.as_bytes());
        let computed = hex::encode(mac.finalize().into_bytes());

        // Constant-time comparison to prevent timing attacks
        if computed.as_bytes().ct_eq(signature.as_bytes()).into() {
            // Signature is valid, proceed
        } else {
            tracing::error!("Firma de webhook de Stripe inválida");
            return Err(PaymentError::InvalidSignature);
        }

        // Parse the event type from the payload
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
