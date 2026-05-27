-- Migration: 20260527000003_create_listing_images
-- Description: Crea la tabla de imágenes asociadas a anuncios
-- Orden: 3/8 (depende de: listings)

CREATE TABLE IF NOT EXISTS listing_images (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id  UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    image_url   TEXT NOT NULL,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Índice para recuperar todas las imágenes de un anuncio eficientemente
CREATE INDEX IF NOT EXISTS idx_listing_images_listing ON listing_images(listing_id);

COMMENT ON TABLE listing_images IS 'Imágenes de un anuncio almacenadas en Cloudinary';
COMMENT ON COLUMN listing_images.listing_id IS 'FK al anuncio propietario; se elimina en cascada si el anuncio se borra';
COMMENT ON COLUMN listing_images.position IS 'Orden de visualización: 0 = imagen principal';
