-- Migration: 000000000000_postgis_extension
-- Description: Activa la extensión PostGIS para geolocalización espacial
-- Orden: 0/9 (debe ejecutarse antes de cualquier migración que use GEOGRAPHY o GEOMETRY)

CREATE EXTENSION IF NOT EXISTS postgis;

COMMENT ON EXTENSION postgis IS 'Extensión espacial PostgreSQL requerida por el módulo geo (ST_DWithin, ST_MakePoint)';
