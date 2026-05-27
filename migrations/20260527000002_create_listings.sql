-- Migration: 20260527000002_create_listings
-- Description: Crea la tabla de anuncios con soporte de geolocalización PostGIS
-- Orden: 2/8 (depende de: users)

CREATE TABLE IF NOT EXISTS listings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    seller_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           TEXT NOT NULL CHECK (length(title) >= 3 AND length(title) <= 100),
    description     TEXT CHECK (length(description) <= 2000),
    price           NUMERIC(10,2) NOT NULL CHECK (price > 0 AND price <= 999999.99),
    currency        TEXT NOT NULL DEFAULT 'eur',
    category        TEXT NOT NULL,
    condition       TEXT NOT NULL CHECK (condition IN ('new', 'like_new', 'used')),
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'sold', 'deleted')),
    location_lat    FLOAT8,
    location_lon    FLOAT8,
    location        GEOGRAPHY(POINT, 4326),
    city            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Índices para búsqueda y filtrado frecuente
CREATE INDEX IF NOT EXISTS idx_listings_seller ON listings(seller_id);
CREATE INDEX IF NOT EXISTS idx_listings_status ON listings(status);
CREATE INDEX IF NOT EXISTS idx_listings_category ON listings(category);
CREATE INDEX IF NOT EXISTS idx_listings_price ON listings(price);
CREATE INDEX IF NOT EXISTS idx_listings_created ON listings(created_at DESC);
-- Índice espacial GiST para búsquedas de proximidad con ST_DWithin (US-15)
CREATE INDEX IF NOT EXISTS idx_listings_location ON listings USING GIST(location);

COMMENT ON TABLE listings IS 'Anuncios de productos publicados por vendedores';
COMMENT ON COLUMN listings.seller_id IS 'FK al usuario propietario del anuncio';
COMMENT ON COLUMN listings.currency IS 'Código ISO 4217 de la moneda (ej. eur, usd)';
COMMENT ON COLUMN listings.condition IS 'Estado físico: new (nuevo), like_new (como nuevo), used (usado)';
COMMENT ON COLUMN listings.status IS 'Estado lógico: active (activo), sold (vendido), deleted (eliminado)';
COMMENT ON COLUMN listings.location IS 'Punto geográfico (longitud, latitud) en SRID 4326 para consultas PostGIS';
