-- Migration: 20260527000008_create_favorites
-- Description: Crea la tabla de anuncios favoritos (relación N:M entre usuarios y anuncios)
-- Orden: 8/8 (depende de: users, listings)

CREATE TABLE IF NOT EXISTS favorites (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    listing_id  UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Garantiza un único favorito por usuario por anuncio (US-16)
    UNIQUE(user_id, listing_id)
);

-- Índice para listar todos los favoritos de un usuario
CREATE INDEX IF NOT EXISTS idx_favorites_user ON favorites(user_id);
-- Índice compuesto para verificar existencia de favorito
CREATE INDEX IF NOT EXISTS idx_favorites_user_listing ON favorites(user_id, listing_id);

COMMENT ON TABLE favorites IS 'Anuncios guardados como favoritos por usuarios';
COMMENT ON COLUMN favorites.user_id IS 'FK al usuario que guarda el favorito; se elimina en cascada si el usuario se borra';
COMMENT ON COLUMN favorites.listing_id IS 'FK al anuncio favorito; se elimina en cascada si el anuncio se borra';
