use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tracks the lifecycle of a payment transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub buyer_id: Uuid,
    pub seller_id: Uuid,
    pub stripe_payment_intent_id: String,
    pub amount_cents: i64,
    pub currency: String,
    pub status: PaymentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Possible states of a payment in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Succeeded,
    Failed,
    Refunded,
}

impl PaymentStatus {
    /// Convert from database text representation.
    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "refunded" => Some(Self::Refunded),
            _ => None,
        }
    }

    /// Convert to database text representation.
    pub fn as_db_string(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Refunded => "refunded",
        }
    }

    /// Returns true if the payment reached a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Refunded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_pending_status_when_is_terminal_then_returns_false() {
        assert!(!PaymentStatus::Pending.is_terminal());
    }

    #[test]
    fn given_succeeded_status_when_is_terminal_then_returns_true() {
        assert!(PaymentStatus::Succeeded.is_terminal());
    }

    #[test]
    fn given_failed_status_when_is_terminal_then_returns_true() {
        assert!(PaymentStatus::Failed.is_terminal());
    }

    #[test]
    fn given_refunded_status_when_is_terminal_then_returns_true() {
        assert!(PaymentStatus::Refunded.is_terminal());
    }

    #[test]
    fn given_valid_db_string_when_from_db_string_then_returns_correct_status() {
        assert_eq!(
            PaymentStatus::from_db_string("pending"),
            Some(PaymentStatus::Pending)
        );
        assert_eq!(
            PaymentStatus::from_db_string("succeeded"),
            Some(PaymentStatus::Succeeded)
        );
        assert_eq!(
            PaymentStatus::from_db_string("failed"),
            Some(PaymentStatus::Failed)
        );
        assert_eq!(
            PaymentStatus::from_db_string("refunded"),
            Some(PaymentStatus::Refunded)
        );
    }

    #[test]
    fn given_invalid_db_string_when_from_db_string_then_returns_none() {
        assert_eq!(PaymentStatus::from_db_string("unknown"), None);
        assert_eq!(PaymentStatus::from_db_string(""), None);
    }

    #[test]
    fn given_status_when_as_db_string_then_returns_correct_string() {
        assert_eq!(PaymentStatus::Pending.as_db_string(), "pending");
        assert_eq!(PaymentStatus::Succeeded.as_db_string(), "succeeded");
        assert_eq!(PaymentStatus::Failed.as_db_string(), "failed");
        assert_eq!(PaymentStatus::Refunded.as_db_string(), "refunded");
    }

    #[test]
    fn given_pending_when_transition_to_terminal_then_status_changes() {
        assert!(!PaymentStatus::Pending.is_terminal());
        assert!(PaymentStatus::Succeeded.is_terminal());
        assert!(PaymentStatus::Failed.is_terminal());
        assert!(PaymentStatus::Refunded.is_terminal());
    }
}
