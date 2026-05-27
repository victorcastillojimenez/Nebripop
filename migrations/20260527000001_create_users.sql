-- Migration: 20260527000001_create_users
-- Description: Activa extensiones PostGIS y uuid-ossp, crea la tabla de usuarios
-- Orden: 1/8 (debe ejecutarse antes que listings, conversations, etc.)

-- 1. Extensiones requeridas
-- PostGIS es necesario para el crate `geo` (ST_DWithin, ST_MakePoint)
-- uuid-ossp proporciona funciones UUID adicionales (aunque gen_random_uuid() está disponible nativamente)
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 2. Tabla users
-- Almacena cuentas de usuario, credenciales, perfil público y métricas de reputación
CREATE TABLE IF NOT EXISTS users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    display_name    TEXT NOT NULL CHECK (length(display_name) >= 2),
    avatar_url      TEXT,
    phone           TEXT,
    role            TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin')),
    rating_avg      NUMERIC(3,2) DEFAULT 0.00 CHECK (rating_avg >= 0 AND rating_avg <= 5),
    total_ratings   INTEGER NOT NULL DEFAULT 0,
    last_login_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Índice único para búsqueda rápida por email en login (US-02)
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

COMMENT ON TABLE users IS 'Usuarios registrados: compradores y vendedores';
COMMENT ON COLUMN users.role IS 'Rol del usuario: user (normal) o admin (administrador)';
COMMENT ON COLUMN users.rating_avg IS 'Promedio de valoraciones recibidas (0.00 - 5.00)';
COMMENT ON COLUMN users.total_ratings IS 'Número total de valoraciones recibidas';
