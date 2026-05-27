-- Migration: 20260527000004_create_conversations
-- Description: Crea la tabla de conversaciones de chat entre comprador y vendedor
-- Orden: 4/8 (depende de: users, listings)

CREATE TABLE IF NOT EXISTS conversations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id      UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    buyer_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    seller_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_message    TEXT,
    last_message_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Garantiza un único hilo de conversación por comprador en cada anuncio (US-09)
    UNIQUE(listing_id, buyer_id)
);

-- Índices para listar conversaciones por usuario (comprador o vendedor)
CREATE INDEX IF NOT EXISTS idx_conversations_buyer ON conversations(buyer_id);
CREATE INDEX IF NOT EXISTS idx_conversations_seller ON conversations(seller_id);
-- Índice para ordenar conversaciones por actividad reciente
CREATE INDEX IF NOT EXISTS idx_conversations_updated ON conversations(updated_at DESC);

COMMENT ON TABLE conversations IS 'Hilos de conversación entre comprador y vendedor vinculados a un anuncio';
COMMENT ON COLUMN conversations.listing_id IS 'FK al anuncio sobre el que se negocia';
COMMENT ON COLUMN conversations.buyer_id IS 'FK al comprador que inició la conversación';
COMMENT ON COLUMN conversations.seller_id IS 'FK al vendedor del anuncio';
COMMENT ON COLUMN conversations.last_message IS 'Contenido textual del último mensaje (para vista previa)';
COMMENT ON COLUMN conversations.last_message_at IS 'Timestamp del último mensaje enviado';
