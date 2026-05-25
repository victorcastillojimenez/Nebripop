---
name: sqlx-best-practices
description: Directrices, mejores prácticas y estándares de codificación para persistencia con SQLx y PostgreSQL en el backend de Nebripop. Utiliza esta skill siempre que vayas a escribir migraciones, queries SQL, repositorios de base de datos, transacciones o tests de base de datos.
---

# SQLx & PostgreSQL Best Practices — Nebripop Backend

Esta skill define las mejores prácticas, directrices de rendimiento y estándares de codificación para interactuar con la base de datos PostgreSQL de **Nebripop** a través de **SQLx**. Su objetivo es garantizar una persistencia segura, eficiente (cumpliendo el NFR de consultas simples en P95 < 200ms), libre de inyecciones de SQL y completamente testeable de forma aislada.

---

## 1. Gestión del Pool de Conexiones (`PgPool`)

El pool de conexiones a PostgreSQL (`PgPool`) debe ser único y administrarse de forma centralizada en el estado global de la aplicación (`AppState`).

### Reglas de Configuración de `PgPool`
1. **Límites de Conexión**: Configurar el tamaño del pool según el entorno (dev: máximo 5 conexiones; prod: máximo 20 conexiones).
2. **Ciclo de Vida**: El pool es seguro de compartir y clonar (`PgPool` internamente es un `Arc`). Debe inyectarse en los handlers y pasarse por referencia a los casos de uso y repositorios (`&PgPool`).
3. **Timeouts**: Establecer timeouts de conexión razonables para evitar cuellos de botella.

### Inicialización en Nebripop
```rust
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

pub async fn create_db_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
        
    let is_prod = std::env::var("APP_ENV").unwrap_or_default() == "production";
    let max_connections = if is_prod { 20 } else { 5 };

    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL")
}
```

---

## 2. Escritura de Queries Tipadas (`query_as!`)

En Nebripop, siempre que sea posible, utilizaremos macros de SQLx que validan las queries en **tiempo de compilación** contra la base de datos PostgreSQL real (usando la variable de entorno `DATABASE_URL` o el archivo offline `.sqlx`).

### Beneficios
* Seguridad de tipos estricta entre las columnas de la BD y los structs de Rust.
* Validación sintáctica y semántica del SQL en tiempo de compilación.
* Detección automática de discrepancias de nulabilidad.

### Mapeo con `sqlx::query_as!`
Para queries que devuelven filas, mapea los resultados directamente a structs de dominio o de persistencia usando `query_as!`.

```rust
use serde::Serialize;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Struct que representa el registro del anuncio en base de datos
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ListingDb {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub title: String,
    pub description: String,
    pub price: rust_decimal::Decimal, // Mapeo exacto de NUMERIC(10,2)
    pub category: String,
    pub condition: String,
    pub status: String,
    pub location_lat: f64,
    pub location_lon: f64,
    pub city: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn find_listing_by_id(id: Uuid, db: &PgPool) -> Result<Option<ListingDb>, sqlx::Error> {
    sqlx::query_as!(
        ListingDb,
        r#"
        SELECT 
            id, seller_id, title, description, price, category, 
            condition, status, location_lat, location_lon, city, 
            created_at, updated_at
        FROM listings 
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(db)
    .await
}
```

### Anulación Forzada de Nulabilidad (Casting de SQLx)
A veces PostgreSQL asume que una columna calculada o con `LEFT JOIN` puede ser NULL, pero en el código sabemos que es obligatoria. Se puede forzar a SQLx a tratarla como no-nula usando `as "column!"` o mapear enums propios de Rust usando `as "column: Type"`.

```rust
#[derive(sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum ListingStatus {
    Active,
    Sold,
    Deleted,
}

pub async fn get_seller_active_listings_count(seller_id: Uuid, db: &PgPool) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*)::bigint as "count!" 
        FROM listings 
        WHERE seller_id = $1 AND status = 'active'
        "#,
        seller_id
    )
    .fetch_one(db)
    .await?;

    Ok(result.count)
}
```

---

## 3. Prevención de Inyecciones de SQL (Parámetros Bind)

> [!CAUTION]
> **Está terminantemente prohibido concatenar strings o usar la macro `format!` para inyectar variables en las consultas SQL.**

Toda variable dinámica debe enlazarse a la query mediante **parámetros bind** (`$1`, `$2`, etc.). Esto permite a PostgreSQL cachear los planes de ejecución de la query y garantiza la inmunidad absoluta frente a ataques de SQL Injection.

### Correcto vs. Incorrecto (Búsqueda básica)

❌ **Incorrecto (Altamente vulnerable a SQL Injection)**
```rust
// Si query contiene "'; DROP TABLE listings; --", se borrará la base de datos!
let query_str = format!("SELECT * FROM listings WHERE title ILIKE '%{}%'", query);
let listings = sqlx::query_as::<_, ListingDb>(&query_str)
    .fetch_all(db)
    .await?;
```

✅ **Correcto (Inmune a SQL Injection mediante placeholders)**
```rust
let search_pattern = format!("%{}%", query);
let listings = sqlx::query_as!(
    ListingDb,
    r#"
    SELECT id, seller_id, title, description, price, category, 
           condition, status, location_lat, location_lon, city, 
           created_at, updated_at
    FROM listings 
    WHERE title ILIKE $1 AND status = 'active'
    "#,
    search_pattern
)
.fetch_all(db)
.await?;
```

---

## 4. Transacciones en Operaciones Multitabla

Cuando una acción de negocio muta múltiples tablas, debe ejecutarse bajo una transacción (`Transaction`) para garantizar la **Atomicidad** (ACID). Si algún paso del flujo falla, la transacción debe hacer un `rollback` automático.

### Flujo Complejo: Crear Pago en Stripe + Confirmar Transacción + Marcar Vendido
En el módulo `payments`, cuando el webhook de Stripe confirma un pago exitoso, debemos:
1. Cambiar el estado del anuncio a `sold`.
2. Crear un registro en la tabla `transactions` como comprobante de compraventa.

```rust
use sqlx::{Postgres, Transaction};

pub async fn complete_purchase_transaction(
    listing_id: Uuid,
    buyer_id: Uuid,
    amount: rust_decimal::Decimal,
    stripe_payment_id: &str,
    db: &PgPool,
) -> Result<(), DomainError> {
    // 1. Iniciar transacción en PostgreSQL
    let mut tx = db.begin().await.map_err(DomainError::DatabaseError)?;

    // 2. Marcar anuncio como vendido
    let rows_affected = sqlx::query!(
        r#"
        UPDATE listings 
        SET status = 'sold', updated_at = now() 
        WHERE id = $1 AND status = 'active'
        "#,
        listing_id
    )
    .execute(&mut *tx)
    .await
    .map_err(DomainError::DatabaseError)?
    .rows_affected();

    // Validar que el anuncio existía y estaba activo
    if rows_affected == 0 {
        // Al salir por error, el objeto `tx` se descarta y SQLx hace ROLLBACK automáticamente
        return Err(DomainError::BadRequest("El anuncio no existe o ya no está activo".to_string()));
    }

    // 3. Obtener el seller_id del anuncio para la transacción
    let listing = sqlx::query!(
        "SELECT seller_id FROM listings WHERE id = $1",
        listing_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(DomainError::DatabaseError)?;

    // 4. Crear el registro de la transacción
    let transaction_id = Uuid::new_v4();
    let platform_fee = amount * rust_decimal::dec!(0.05); // 5% de comisión de Nebripop

    sqlx::query!(
        r#"
        INSERT INTO transactions (id, listing_id, buyer_id, seller_id, amount, platform_fee, stripe_payment_id, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'paid')
        "#,
        transaction_id,
        listing_id,
        buyer_id,
        listing.seller_id,
        amount,
        platform_fee,
        stripe_payment_id
    )
    .execute(&mut *tx)
    .await
    .map_err(DomainError::DatabaseError)?;

    // 5. Confirmar transacción (COMMIT en la BD)
    tx.commit().await.map_err(DomainError::DatabaseError)?;

    Ok(())
}
```

---

## 5. Manejo de Errores y Conversión a Errores de Dominio

Los errores técnicos de base de datos (`sqlx::Error`) no deben filtrarse nunca a las capas superiores del dominio ni a las respuestas HTTP JSON. Deben capturarse, registrarse mediante logs (`tracing::error!`) y convertirse a un enum de error de dominio propio.

### Mapeo de Códigos de Error de Postgres
PostgreSQL expone códigos de error estándar (SQLSTATE). Los más habituales a interceptar en Nebripop son:
* **`23505` (unique_violation)**: Intento de registrar un email ya existente.
* **`23503` (foreign_key_violation)**: Agregar un favorito para un anuncio inexistente.

### Implementación del Mapeador de Errores

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("El recurso solicitado no existe")]
    NotFound,
    
    #[error("Ya existe un registro con estos datos: {0}")]
    AlreadyExists(String),
    
    #[error("Conflicto de integridad en base de datos: {0}")]
    ForeignKeyViolation(String),
    
    #[error("Error interno del servidor")]
    DatabaseError(#[source] sqlx::Error),
    
    #[error("Acción incorrecta: {0}")]
    BadRequest(String),
}

// Convertidor de errores SQLx a errores de Dominio
pub fn map_sqlx_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            let code = db_err.code().unwrap_or_default();
            match code.as_ref() {
                "23505" => {
                    // Intento de duplicidad (ej. email)
                    let constraint = db_err.constraint().unwrap_or("campo duplicado");
                    DomainError::AlreadyExists(format!("Violación de unicidad en {}", constraint))
                }
                "23503" => {
                    // Violación de clave externa (ej. FK inexistente)
                    let constraint = db_err.constraint().unwrap_or("clave externa");
                    DomainError::ForeignKeyViolation(format!("Integridad violada en {}", constraint))
                }
                _ => {
                    tracing::error!("Error de base de datos sin clasificar: [SQLSTATE {}] {:?}", code, db_err);
                    DomainError::DatabaseError(sqlx::Error::Database(db_err))
                }
            }
        }
        other => {
            tracing::error!("Error de conexión o de red en SQLx: {:?}", other);
            DomainError::DatabaseError(other)
        }
    }
}
```

---

## 6. Diseño de Índices para las Queries Frecuentes

Para cumplir con el requisito de latencia **P95 < 200ms** del PRD, las búsquedas recurrentes no deben hacer escaneos de tabla completa (`Seq Scan`). Debemos crear índices explícitos en nuestras migraciones de PostgreSQL.

### Índices Críticos del Modelo de Datos de Nebripop

| Tabla | Columna | Tipo de Índice | Razón y Query Frecuente del PRD |
|-------|---------|----------------|---------------------------------|
| `users` | `email` | **B-Tree (Unique)** | Login frecuente (`US-02`) y registro de unicidad |
| `listings` | `seller_id` | **B-Tree** | Listar anuncios del vendedor propio (`US-05`, `US-06`) |
| `listings` | `category` | **B-Tree** | Filtrado por categorías (`US-08`) |
| `listings` | `status` | **B-Tree** | Excluir anuncios vendidos de búsquedas activas (`US-06`) |
| `listings` | `created_at` | **B-Tree (Desc)** | Ordenar feed principal por anuncios recientes |
| `listings` | `(location_lat, location_lon)` | **GiST / B-Tree compuesto** | Geolocalización de cercanía por Haversine (`US-15`) |
| `listing_images` | `listing_id` | **B-Tree** | Recuperar imágenes del detalle del anuncio (`US-04`) |
| `conversations` | `(listing_id, buyer_id)` | **B-Tree (Unique)** | Asegurar un único chat por comprador en cada anuncio (`US-09`) |
| `messages` | `conversation_id` | **B-Tree** | Cargar histórico de mensajes del chat (`US-10`) |
| `favorites` | `(user_id, listing_id)` | **B-Tree (Unique)** | Comprobación y carga de anuncios favoritos del usuario (`US-16`) |

### Script de Migración PostgreSQL (`migrations/20260522000001_create_indices.sql`)
```sql
-- 1. Usuarios e Identidad
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- 2. Anuncios (Listings)
CREATE INDEX IF NOT EXISTS idx_listings_seller_id ON listings(seller_id);
CREATE INDEX IF NOT EXISTS idx_listings_category ON listings(category) WHERE status = 'active';
CREATE INDEX IF NOT EXISTS idx_listings_status ON listings(status);
CREATE INDEX IF NOT EXISTS idx_listings_created_at_desc ON listings(created_at DESC);

-- 3. Índice compuesto para localización Haversine rápida en anuncios activos
CREATE INDEX IF NOT EXISTS idx_listings_geo ON listings(location_lat, location_lon) WHERE status = 'active';

-- 4. Imágenes
CREATE INDEX IF NOT EXISTS idx_images_listing_id ON listing_images(listing_id);

-- 5. Chats y Mensajería
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_listing_buyer ON conversations(listing_id, buyer_id);
CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);

-- 6. Favoritos (Muchos a Muchos)
CREATE UNIQUE INDEX IF NOT EXISTS idx_favorites_user_listing ON favorites(user_id, listing_id);
CREATE INDEX IF NOT EXISTS idx_favorites_user_id ON favorites(user_id);
```

---

## 7. Tests de Integración con `sqlx::test`

No debemos mockear las llamadas a base de datos. SQLx proporciona una macro `#[sqlx::test]` extremadamente potente que gestiona de manera transparente bases de datos efímeras (sandboxed) por cada hilo de ejecución de test, aplicando las migraciones automáticamente.

### Ventajas:
* Cada test corre en una base de datos PostgreSQL limpia e independiente.
* No hay interferencia de datos entre tests concurrentes.
* No requiere levantar ni limpiar mocks manuales.

### Ejemplo de Test de Integración para Listings

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use rust_decimal::dec;

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_and_find_listing(pool: PgPool) {
        // 1. Crear un usuario semilla (requerido por la clave foránea de listings)
        let seller_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, email, password_hash, display_name) VALUES ($1, $2, $3, $4)",
            seller_id,
            "test_seller@nebrija.es",
            "hashed_pw_here",
            "Vendedor Test"
        )
        .execute(&pool)
        .await
        .unwrap();

        // 2. Preparar los datos del anuncio
        let listing_id = Uuid::new_v4();
        let title = "Bicicleta de montaña BH".to_string();
        let description = "En perfecto estado, poco uso".to_string();
        let price = dec!(150.00);
        let category = "sports".to_string();
        let condition = "like_new".to_string();
        let lat = 40.4167; // Madrid
        let lon = -3.7037;

        // 3. Ejecutar la query de creación
        sqlx::query!(
            r#"
            INSERT INTO listings (id, seller_id, title, description, price, category, condition, location_lat, location_lon, city)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'Madrid')
            "#,
            listing_id,
            seller_id,
            title,
            description,
            price,
            category,
            condition,
            lat,
            lon
        )
        .execute(&pool)
        .await
        .unwrap();

        // 4. Verificar recuperación tipada con find_listing_by_id
        let retrieved = find_listing_by_id(listing_id, &pool).await.unwrap();
        
        assert!(retrieved.is_some());
        let listing = retrieved.unwrap();
        assert_eq!(listing.title, "Bicicleta de montaña BH");
        assert_eq!(listing.price, price);
        assert_eq!(listing.status, "active"); // Estado por defecto
    }
}
```

---

## 8. Patrones Correctos vs. Incorrectos (Ejemplos)

### A. Consultas con Retorno Opcional

❌ **Incorrecto (Uso de `fetch_one` cuando el ID puede no existir; causa pánico o `RowNotFound` no controlado)**
```rust
pub async fn get_user_by_id_unsafe(id: Uuid, db: &PgPool) -> Result<UserDb, sqlx::Error> {
    // Si el usuario no existe, esto explota con RowNotFound!
    sqlx::query_as!(UserDb, "SELECT * FROM users WHERE id = $1", id)
        .fetch_one(db)
        .await
}
```

✅ **Correcto (Uso de `fetch_optional` y retorno de `Option<T>`)**
```rust
pub async fn get_user_by_id_safe(id: Uuid, db: &PgPool) -> Result<Option<UserDb>, sqlx::Error> {
    sqlx::query_as!(UserDb, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(db)
        .await
}
```

---

### B. Evitar el Problema de Consultas N+1 (Relaciones)

Para cargar un anuncio con sus imágenes asociadas, no hagas consultas secuenciales sueltas en bucle.

❌ **Incorrecto (N+1 queries: una query para listings y N queries en bucle para imágenes)**
```rust
pub async fn get_listings_with_images_n_plus_one(db: &PgPool) -> Result<Vec<ListingWithImages>, sqlx::Error> {
    let listings = sqlx::query_as!(ListingDb, "SELECT * FROM listings WHERE status = 'active'")
        .fetch_all(db)
        .await?;

    let mut result = Vec::new();
    for listing in listings {
        // ¡Query en bucle! Causa latencias enormes (violación crítica de NFR < 200ms)
        let images = sqlx::query!("SELECT image_url FROM listing_images WHERE listing_id = $1", listing.id)
            .fetch_all(db)
            .await?;
        
        result.push(ListingWithImages { listing, images });
    }
    Ok(result)
}
```

✅ **Correcto (Agrupar en SQL usando `LEFT JOIN` o agregación JSON de Postgres)**
```rust
#[derive(sqlx::FromRow)]
pub struct ListingWithImagesRow {
    pub id: Uuid,
    pub title: String,
    pub price: rust_decimal::Decimal,
    // Carga todas las URLs asociadas agregadas en formato de array JSON
    pub images_json: serde_json::Value,
}

pub async fn get_listings_with_images_efficient(db: &PgPool) -> Result<Vec<ListingWithImagesRow>, sqlx::Error> {
    sqlx::query_as!(
        ListingWithImagesRow,
        r#"
        SELECT 
            l.id, l.title, l.price,
            COALESCE(json_agg(li.image_url) FILTER (WHERE li.image_url IS NOT NULL), '[]'::json) as "images_json!"
        FROM listings l
        LEFT JOIN listing_images li ON l.id = li.listing_id
        WHERE l.status = 'active'
        GROUP BY l.id, l.title, l.price
        "#
    )
    .fetch_all(db)
    .await
}
```

---

## 9. Las 12 Reglas Críticas de SQLx para Nebripop

1. **Uso Exclusivo de Macros Validadas**: Emplea siempre `query!`, `query_as!` y `query_scalar!` para validar tus estructuras en tiempo de compilación. Solo usa queries de ejecución en runtime si vas a componer filtros altamente dinámicos en tiempo de ejecución.
2. **Prohibición de Concatenación**: No uses `format!`, `concat!` o interpolación de strings para construir queries dinámicas. Usa placeholders de parámetros bind `$1`, `$2`... para proteger a Nebripop contra inyección SQL.
3. **Mapeo de Tipos de Datos Correctos**: Mapea siempre el tipo de base de datos PostgreSQL `NUMERIC` al tipo `rust_decimal::Decimal` de Rust. **Nunca** utilices números de coma flotante (`f64` o `f32`) para representar precios de anuncios o transacciones Stripe.
4. **Ciclo de Vida del Pool Centralizado**: Inyecta el pool de base de datos (`PgPool`) una única vez en `AppState` al levantar el backend. Transmítelo por referencia simple (`&PgPool`) a las capas profundas de repositorio de base de datos.
5. **No Exponer sqlx::Error**: Convierte todos los fallos del driver PostgreSQL al enum de error del dominio (`DomainError`) usando adaptadores dedicados. La lógica del dominio de Nebripop nunca debe importar ni acoplarse a `sqlx::Error`.
6. **Mapeo de Errores de Restricciones**: Intercepta explícitamente códigos de error críticos de Postgres (ej. `23505` para duplicados de email, `23503` para dependencias inexistentes en favoritos) para retornar errores descriptivos al dominio en lugar de panics.
7. **Control de Nulabilidad en Macros**: Utiliza la sintaxis de sobreescritura de SQLx (`as "campo!"`) para forzar tipos no nulos en columnas donde SQLx no puede inferir la nulabilidad correctamente debido a subconsultas o agregaciones.
8. **Transacciones en Escrituras Múltiples**: Cualquier caso de uso que afecte a más de una tabla en cascada (ej. marcar anuncio vendido y registrar transacción de Stripe) **debe** realizarse dentro de un bloque transaccional (`pool.begin().await?`).
9. **Cero Queries en Bucle (Problema N+1)**: Está prohibido realizar consultas de base de datos dentro de un bucle `for` o `while` en Rust. Utiliza sentencias SQL unificadas con `LEFT JOIN`, cláusulas `IN` o agregación `json_agg` de PostgreSQL.
10. **Índices en Claves Foráneas**: Toda clave externa definida en nuestras migraciones SQL (como `seller_id`, `listing_id`, `buyer_id`) debe contar obligatoriamente con un índice B-Tree creado para optimizar cruces y lecturas en cascada.
11. **Haversine Integrado en Postgres**: Las búsquedas geolocalizadas por radio de cercanía deben calcularse dentro de la propia sentencia SQL mediante la fórmula matemática de Haversine y apoyarse en el índice compuesto `idx_listings_geo` para garantizar latencias bajas.
12. **Tests Aislados con Base de Datos**: Ejecuta todos los tests de persistencia bajo la macro de test de integración asíncrona `#[sqlx::test(migrations = "./migrations")]` para asegurar aislamiento completo y evitar efectos colaterales de concurrencia.
