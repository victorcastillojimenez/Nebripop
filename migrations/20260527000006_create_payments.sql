-- Migration: 20260527000006_create_payments
-- Description: Crea la tabla de pagos/transacciones vinculadas a Stripe
-- Orden: 6/8 (depende de: users, listings)

CREATE TABLE IF NOT EXISTS payments (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id                  UUID NOT NULL REFERENCES listings(id),
    buyer_id                    UUID NOT NULL REFERENCES users(id),
    seller_id                   UUID NOT NULL REFERENCES users(id),
    stripe_payment_intent_id    TEXT NOT NULL UNIQUE,
    amount_cents                BIGINT NOT NULL CHECK (amount_cents > 0),
    currency                    TEXT NOT NULL DEFAULT 'eur',
    status                      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'succeeded', 'failed', 'refunded')),
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Índices para consultas por comprador, vendedor, anuncio y referencia Stripe
CREATE INDEX IF NOT EXISTS idx_payments_buyer ON payments(buyer_id);
CREATE INDEX IF NOT EXISTS idx_payments_seller ON payments(seller_id);
CREATE INDEX IF NOT EXISTS idx_payments_listing ON payments(listing_id);
CREATE INDEX IF NOT EXISTS idx_payments_stripe ON payments(stripe_payment_intent_id);

COMMENT ON TABLE payments IS 'Transacciones de pago procesadas a través de Stripe';
COMMENT ON COLUMN payments.listing_id IS 'FK al anuncio comprado (sin CASCADE para preservar registro histórico)';
COMMENT ON COLUMN payments.buyer_id IS 'FK al comprador que realiza el pago';
COMMENT ON COLUMN payments.seller_id IS 'FK al vendedor que recibe el pago';
COMMENT ON COLUMN payments.stripe_payment_intent_id IS 'ID único del PaymentIntent en Stripe para trazabilidad';
COMMENT ON COLUMN payments.amount_cents IS 'Monto en céntimos (ej. 1500 = 15.00 EUR)';
COMMENT ON COLUMN payments.status IS 'Estado: pending (pendiente), succeeded (exitoso), failed (fallido), refunded (reembolsado)';
