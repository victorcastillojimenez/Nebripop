use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to create a new PaymentIntent.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIntentDto {
    pub listing_id: Uuid,
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "eur".to_string()
}

/// Response returned after successfully creating a PaymentIntent.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIntentResponse {
    pub payment_id: Uuid,
    pub client_secret: String,
}

/// Status information for a payment.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentStatusDto {
    pub id: Uuid,
    pub status: String,
    pub amount_cents: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

/// Generic webhook acknowledgement response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookResponse {
    pub received: bool,
}
