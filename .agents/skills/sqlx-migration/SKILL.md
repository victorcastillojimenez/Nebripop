# Skill: SQLx Migrations for Nebripop

Esta skill define el estándar para la gestión del esquema de base de datos en Nebripop usando SQLx. Asegura la integridad referencial, tipos de datos óptimos e índices para alta performance (<200ms).

## Contexto
Según el **PRD (Sección 6)**, existen 8 entidades conectadas. El orden de creación es crítico: `users` -> `listings` -> dependencias. Usamos PostgreSQL 15+ con UUIDs nativos.

## Reglas y Ejemplos

### 1. Convención de Nomenclatura
Los archivos deben seguir el formato `YYYYMMDDHHMMSS_descripcion.sql` para garantizar el orden cronológico.

### 2. Estructura de "Up" e Idempotencia
Cada migración debe ser idempotente y manejar transacciones explícitas si es necesario.

```sql
-- migration.sql
CREATE TABLE IF NOT EXISTS table_name (
    -- definition
);
```

### 3. Tipos de Datos Estándar
- **Identificadores**: `UUID PRIMARY KEY DEFAULT gen_random_uuid()`.
- **Dinero**: `NUMERIC(10,2) CHECK (price > 0)`. Nunca usar FLOAT.
- **Fechas**: `TIMESTAMPTZ DEFAULT now()`. Siempre con zona horaria.
- **Texto**: `VARCHAR(N)` para límites estrictos, `TEXT` para descripciones largas.

### 4. Estrategia de Índices
Basado en las queries del PRD, los índices son obligatorios en:
- `listings(category, status)`: Para filtros rápidos en el feed.
- `listings(created_at)`: Para ordenación cronológica.
- `conversations(updated_at)`: Para mostrar los chats más recientes primero.
- `favorites(user_id, listing_id)`: Índice compuesto único.

---

## Migraciones Completas (8 Entidades)

### 20260525000001_initial_schema.sql

```sql
-- 1. Users
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    avatar_url TEXT,
    bio VARCHAR(500),
    location_lat FLOAT8,
    location_lon FLOAT8,
    city VARCHAR(100),
    avg_rating FLOAT4 NOT NULL DEFAULT 0.0,
    rating_count INT4 NOT NULL DEFAULT 0,
    role VARCHAR(20) NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2. Listings
CREATE TABLE IF NOT EXISTS listings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    seller_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(150) NOT NULL,
    description TEXT NOT NULL,
    price NUMERIC(10,2) NOT NULL CHECK (price > 0),
    category VARCHAR(50) NOT NULL,
    condition VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    location_lat FLOAT8 NOT NULL,
    location_lon FLOAT8 NOT NULL,
    city VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_listings_category_status ON listings(category, status);
CREATE INDEX IF NOT EXISTS idx_listings_created_at ON listings(created_at DESC);

-- 3. Listing Images
CREATE TABLE IF NOT EXISTS listing_images (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    image_url TEXT NOT NULL,
    position INT2 NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_listing_images_listing_id ON listing_images(listing_id);

-- 4. Conversations
CREATE TABLE IF NOT EXISTS conversations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    buyer_id UUID NOT NULL REFERENCES users(id),
    seller_id UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(listing_id, buyer_id)
);
CREATE INDEX IF NOT EXISTS idx_conversations_updated_at ON conversations(updated_at DESC);

-- 5. Messages
CREATE TABLE IF NOT EXISTS messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id),
    content TEXT NOT NULL CHECK (length(content) > 0),
    is_read BOOL NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 6. Transactions
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES listings(id),
    buyer_id UUID NOT NULL REFERENCES users(id),
    seller_id UUID NOT NULL REFERENCES users(id),
    amount NUMERIC(10,2) NOT NULL CHECK (amount > 0),
    platform_fee NUMERIC(10,2) NOT NULL DEFAULT 0.00,
    stripe_payment_id VARCHAR(255) UNIQUE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 7. Ratings
CREATE TABLE IF NOT EXISTS ratings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID NOT NULL REFERENCES transactions(id),
    rater_id UUID NOT NULL REFERENCES users(id),
    rated_id UUID NOT NULL REFERENCES users(id),
    score INT2 NOT NULL CHECK (score BETWEEN 1 AND 5),
    comment VARCHAR(500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(transaction_id, rater_id)
);

-- 8. Favorites
CREATE TABLE IF NOT EXISTS favorites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, listing_id)
);
```

### 5. Datos Iniciales (Seed)
Añade una migración específica para catálogos que no cambian frecuentemente.

```sql
INSERT INTO listings (seller_id, title, description, price, category, condition, location_lat, location_lon, city)
-- Solo para testing inicial o categorías si existiera una tabla maestra
```

### 6. Ejecución y Control
- **Aplicar**: `sqlx migrate run`
- **Revertir**: `sqlx migrate revert` (solo revierte la última).
- **Estado**: `sqlx migrate info`

## Recomendaciones de Desarrollo
- **Consistencia de Nomenclatura**: Siempre usa `snake_case` para tablas y columnas.
- **Check Constraints**: Úsalos para asegurar que el precio sea positivo o el score esté entre 1 y 5.
- **Cascada**: Usa `ON DELETE CASCADE` solo en relaciones de pertenencia estricta (ej: fotos de un anuncio). En transacciones, usa `RESTRICT` para evitar pérdida de datos financieros.
