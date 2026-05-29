use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::PaymentError;
use crate::models::{Payment, PaymentStatus};
use crate::ports::PaymentRepository;

/// Private row struct matching the `payments` table schema for SQLx.
#[derive(Debug, sqlx::FromRow)]
struct PaymentRow {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub buyer_id: Uuid,
    pub seller_id: Uuid,
    pub stripe_payment_intent_id: String,
    pub amount_cents: i64,
    pub currency: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PaymentRow> for Payment {
    fn from(row: PaymentRow) -> Self {
        Self {
            id: row.id,
            listing_id: row.listing_id,
            buyer_id: row.buyer_id,
            seller_id: row.seller_id,
            stripe_payment_intent_id: row.stripe_payment_intent_id,
            amount_cents: row.amount_cents,
            currency: row.currency,
            status: PaymentStatus::from_db_string(&row.status)
                .unwrap_or(PaymentStatus::Pending),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL-backed implementation of `PaymentRepository`.
#[derive(Clone)]
pub struct PostgresPaymentRepository {
    pool: PgPool,
}

impl PostgresPaymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PaymentRepository for PostgresPaymentRepository {
    async fn insert(&self, payment: &Payment) -> Result<Payment, PaymentError> {
        let status_str = payment.status.as_db_string();
        let row = sqlx::query_as::<_, PaymentRow>(
            r#"
            INSERT INTO payments (id, listing_id, buyer_id, seller_id, stripe_payment_intent_id, amount_cents, currency, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, listing_id, buyer_id, seller_id, stripe_payment_intent_id, amount_cents, currency, status, created_at, updated_at
            "#,
        )
        .bind(payment.id)
        .bind(payment.listing_id)
        .bind(payment.buyer_id)
        .bind(payment.seller_id)
        .bind(&payment.stripe_payment_intent_id)
        .bind(payment.amount_cents)
        .bind(&payment.currency)
        .bind(status_str)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn update_status(
        &self,
        stripe_intent_id: &str,
        new_status: PaymentStatus,
    ) -> Result<(), PaymentError> {
        let status_str = new_status.as_db_string();
        let result = sqlx::query(
            r#"
            UPDATE payments
            SET status = $1, updated_at = now()
            WHERE stripe_payment_intent_id = $2
            "#,
        )
        .bind(status_str)
        .bind(stripe_intent_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            tracing::warn!(
                "No payment found for stripe_intent_id: {}",
                stripe_intent_id
            );
        }

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Payment>, PaymentError> {
        let row = sqlx::query_as::<_, PaymentRow>(
            r#"
            SELECT id, listing_id, buyer_id, seller_id, stripe_payment_intent_id, amount_cents, currency, status, created_at, updated_at
            FROM payments
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn find_by_stripe_intent_id(
        &self,
        stripe_intent_id: &str,
    ) -> Result<Option<Payment>, PaymentError> {
        let row = sqlx::query_as::<_, PaymentRow>(
            r#"
            SELECT id, listing_id, buyer_id, seller_id, stripe_payment_intent_id, amount_cents, currency, status, created_at, updated_at
            FROM payments
            WHERE stripe_payment_intent_id = $1
            "#,
        )
        .bind(stripe_intent_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }
}
