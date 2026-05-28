use uuid::Uuid;

use crate::dtos::{CreateIntentDto, CreateIntentResponse};
use crate::errors::PaymentError;
use crate::models::{Payment, PaymentStatus};
use crate::ports::{PaymentRepository, StripePort};

/// Creates a Stripe PaymentIntent for a listing and persists the payment record.
pub async fn create_intent_usecase(
    dto: CreateIntentDto,
    buyer_id: Uuid,
    listing_service: &dyn ListingService,
    payment_repo: &dyn PaymentRepository,
    stripe_port: &dyn StripePort,
) -> Result<CreateIntentResponse, PaymentError> {
    // 1. Verify the listing exists and is active
    let listing = listing_service
        .find_active_by_id(dto.listing_id)
        .await?
        .ok_or(PaymentError::ListingNotAvailable(dto.listing_id))?;

    // 2. Prevent self-purchase
    if listing.seller_id == buyer_id {
        return Err(PaymentError::SelfPurchase);
    }

    // 3. Create the PaymentIntent in Stripe
    let (intent_id, client_secret) = stripe_port
        .create_payment_intent(
            listing.amount_cents,
            &dto.currency,
            dto.listing_id,
            buyer_id,
        )
        .await?;

    // 4. Persist the payment with status Pending
    let payment = Payment {
        id: Uuid::new_v4(),
        listing_id: dto.listing_id,
        buyer_id,
        seller_id: listing.seller_id,
        stripe_payment_intent_id: intent_id,
        amount_cents: listing.amount_cents,
        currency: dto.currency,
        status: PaymentStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let saved = payment_repo.insert(&payment).await?;

    Ok(CreateIntentResponse {
        payment_id: saved.id,
        client_secret,
    })
}

/// Trait for listing operations needed by the usecase.
/// This avoids direct coupling to the listings crate.
#[async_trait::async_trait]
pub trait ListingService: Send + Sync {
    /// Find an active listing by ID. Returns the seller_id and amount in cents.
    async fn find_active_by_id(&self, id: Uuid) -> Result<Option<ListingInfo>, PaymentError>;
}

/// Minimal listing info needed for payment creation.
#[derive(Debug, Clone)]
pub struct ListingInfo {
    pub seller_id: Uuid,
    pub amount_cents: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::dtos::CreateIntentDto;
    use crate::ports::{PaymentRepository, StripePort};
    use crate::models::Payment;

    // --- Mock implementations ---

    struct MockListingService {
        listing: Option<ListingInfo>,
    }

    #[async_trait]
    impl ListingService for MockListingService {
        async fn find_active_by_id(&self, _id: Uuid) -> Result<Option<ListingInfo>, PaymentError> {
            Ok(self.listing.clone())
        }
    }

    struct MockPaymentRepository;

    #[async_trait]
    impl PaymentRepository for MockPaymentRepository {
        async fn insert(&self, _payment: &Payment) -> Result<Payment, PaymentError> {
            Ok(Payment {
                id: Uuid::new_v4(),
                listing_id: Uuid::new_v4(),
                buyer_id: Uuid::new_v4(),
                seller_id: Uuid::new_v4(),
                stripe_payment_intent_id: "pi_mock".to_string(),
                amount_cents: 1000,
                currency: "eur".to_string(),
                status: PaymentStatus::Pending,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        async fn update_status(&self, _stripe_intent_id: &str, _new_status: PaymentStatus) -> Result<(), PaymentError> {
            Ok(())
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<Payment>, PaymentError> {
            Ok(None)
        }

        async fn find_by_stripe_intent_id(&self, _stripe_intent_id: &str) -> Result<Option<Payment>, PaymentError> {
            Ok(None)
        }
    }

    struct MockStripePort;

    #[async_trait]
    impl StripePort for MockStripePort {
        async fn create_payment_intent(
            &self,
            _amount_cents: i64,
            _currency: &str,
            _listing_id: Uuid,
            _buyer_id: Uuid,
        ) -> Result<(String, String), PaymentError> {
            Ok(("pi_mock".to_string(), "cs_mock".to_string()))
        }

        fn verify_webhook_signature(&self, _payload: &[u8], _signature_header: &str) -> Result<String, PaymentError> {
            Ok("payment_intent.succeeded".to_string())
        }
    }

    // --- Self-purchase tests ---

    #[tokio::test]
    async fn given_buyer_is_seller_when_create_intent_then_returns_self_purchase_error() {
        let buyer_id = Uuid::new_v4();
        let seller_id = buyer_id; // Same user!
        let listing_id = Uuid::new_v4();

        let listing_service = MockListingService {
            listing: Some(ListingInfo {
                seller_id,
                amount_cents: 5000,
            }),
        };

        let dto = CreateIntentDto {
            listing_id,
            currency: "eur".to_string(),
        };

        let result = create_intent_usecase(
            dto,
            buyer_id,
            &listing_service,
            &MockPaymentRepository,
            &MockStripePort,
        ).await;

        assert!(matches!(result, Err(PaymentError::SelfPurchase)));
    }

    #[tokio::test]
    async fn given_buyer_is_not_seller_when_create_intent_then_succeeds() {
        let buyer_id = Uuid::new_v4();
        let seller_id = Uuid::new_v4(); // Different user
        let listing_id = Uuid::new_v4();

        let listing_service = MockListingService {
            listing: Some(ListingInfo {
                seller_id,
                amount_cents: 5000,
            }),
        };

        let dto = CreateIntentDto {
            listing_id,
            currency: "eur".to_string(),
        };

        let result = create_intent_usecase(
            dto,
            buyer_id,
            &listing_service,
            &MockPaymentRepository,
            &MockStripePort,
        ).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.client_secret, "cs_mock");
    }

    #[tokio::test]
    async fn given_listing_not_found_when_create_intent_then_returns_listing_not_available() {
        let buyer_id = Uuid::new_v4();

        let listing_service = MockListingService { listing: None };

        let dto = CreateIntentDto {
            listing_id: Uuid::new_v4(),
            currency: "eur".to_string(),
        };

        let result = create_intent_usecase(
            dto,
            buyer_id,
            &listing_service,
            &MockPaymentRepository,
            &MockStripePort,
        ).await;

        assert!(matches!(result, Err(PaymentError::ListingNotAvailable(_))));
    }
}
