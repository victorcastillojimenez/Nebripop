-- Migration: 20260527000005_create_messages
-- Description: Crea la tabla de mensajes individuales dentro de conversaciones
-- Orden: 5/8 (depende de: conversations, users)

CREATE TABLE IF NOT EXISTS messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content         TEXT NOT NULL CHECK (length(content) >= 1 AND length(content) <= 5000),
    is_read         BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Índices para carga de histórico y detección de mensajes no leídos
CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);
CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at ASC);
-- Índice parcial para contar mensajes no leídos eficientemente (US-10)
CREATE INDEX IF NOT EXISTS idx_messages_unread ON messages(conversation_id, is_read) WHERE is_read = false;

COMMENT ON TABLE messages IS 'Mensajes individuales dentro de una conversación de chat';
COMMENT ON COLUMN messages.conversation_id IS 'FK a la conversación padre; se elimina en cascada';
COMMENT ON COLUMN messages.sender_id IS 'FK al usuario que envió el mensaje';
COMMENT ON COLUMN messages.is_read IS 'Indica si el destinatario ha leído el mensaje';
