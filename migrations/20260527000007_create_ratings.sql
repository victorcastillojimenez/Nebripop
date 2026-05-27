-- Migration: 20260527000007_create_ratings
-- Description: Crea la tabla de valoraciones post-transacción entre usuarios
-- Orden: 7/8 (depende de: users, listings)

CREATE TABLE IF NOT EXISTS ratings (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id  UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    rater_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rated_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    score       SMALLINT NOT NULL CHECK (score >= 1 AND score <= 5),
    comment     TEXT CHECK (length(comment) <= 500),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Garantiza una sola valoración por parte por transacción (US-12)
    UNIQUE(listing_id, rater_id)
);

-- Índices para consultar valoraciones recibidas por usuario y por anuncio
CREATE INDEX IF NOT EXISTS idx_ratings_rated ON ratings(rated_id);
CREATE INDEX IF NOT EXISTS idx_ratings_listing ON ratings(listing_id);

COMMENT ON TABLE ratings IS 'Valoraciones de 1-5 estrellas entre usuarios tras una transacción';
COMMENT ON COLUMN ratings.listing_id IS 'FK al anuncio asociado a la valoración';
COMMENT ON COLUMN ratings.rater_id IS 'FK al usuario que emite la valoración (quien puntúa)';
COMMENT ON COLUMN ratings.rated_id IS 'FK al usuario que recibe la valoración (quien es puntuado)';
COMMENT ON COLUMN ratings.score IS 'Puntuación: 1 (mínimo) a 5 (máximo)';
COMMENT ON COLUMN ratings.comment IS 'Comentario opcional asociado a la valoración (máx. 500 caracteres)';
