use async_trait::async_trait;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::PaymentError;
use crate::usecases::create_intent_usecase::{ListingInfo, ListingService};

/// Adapter that retrieves listing data from PostgreSQL for payment usecases.
#[derive(Clone)]
pub struct PaymentsListingAdapter {
    pool: PgPool,
}

impl PaymentsListingAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ListingService for PaymentsListingAdapter {
    async fn find_active_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<ListingInfo>, PaymentError> {
        let row = sqlx::query_as::<_, (Uuid, sqlx::types::Decimal)>(
            r#"
            SELECT seller_id, price
            FROM listings
            WHERE id = $1 AND status = 'active'
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(PaymentError::from)?;

        match row {
            Some((seller_id, price)) => {
                // Convert price (e.g. 150.00) to cents (15000)
                let hundred = Decimal::new(100, 0);
                let amount_cents = (price * hundred)
                    .round()
                    .to_i64()
                    .ok_or_else(|| {
                        PaymentError::StripeError(
                            "El precio del anuncio no cabe en un entero de 64 bits".to_string(),
                        )
                    })?;
                Ok(Some(ListingInfo {
                    seller_id,
                    amount_cents,
                }))
            }
            None => Ok(None),
        }
    }
}
