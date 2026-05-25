# Skill: MeiliSearch Integration for Nebripop

Esta skill define el estándar para la búsqueda full-text de Nebripop, integrando MeiliSearch con PostgreSQL y Axum, asegurando velocidad (<300ms) y consistencia.

## Contexto
Según el **PRD (Módulo search)**, la búsqueda es una funcionalidad core. Se utiliza MeiliSearch para indexación avanzada y PostgreSQL como fuente de verdad. La arquitectura debe soportar fallos del servicio de búsqueda mediante un fallback a SQL.

## Reglas y Ejemplos

### 1. Configuración del Índice `listings`
Al iniciar el servicio, asegúrate de que el índice existe y tiene configurados los atributos necesarios.

```rust
pub async fn setup_meilisearch(client: &Client) -> Result<(), AppError> {
    let index = client.index("listings");
    
    // Configurar atributos filtrables
    index.set_filterable_attributes([
        "category", "price", "status", "city", "_geo"
    ]).await?;

    // Configurar atributos ordenables
    index.set_sortable_attributes([
        "price", "created_at"
    ]).await?;

    Ok(())
}
```

### 2. Atributos Filtrables y Geo-búsqueda
MeiliSearch requiere el campo especial `_geo` para filtros de distancia. Mapea `location_lat` y `location_lon` de PostgreSQL a este objeto.

```json
{
  "id": "uuid-123",
  "title": "Bicicleta montaña",
  "category": "deportes",
  "price": 150.0,
  "status": "active",
  "_geo": {
    "lat": 40.4168,
    "lng": -3.7038
  }
}
```

### 3. Pesos de Relevancia (Ranking)
Configura los campos para que los aciertos en el título tengan más peso que en la descripción o categoría.

```rust
index.set_searchable_attributes([
    "title", "category", "description"
]).await?;

// Orden de importancia para MeiliSearch
// 1. Exactness (Palabra exacta)
// 2. Attribute (Donde aparece: title > description)
// 3. Proximity
```

### 4. Sincronización en Tiempo Real (Upsert)
Cada vez que un anuncio se crea o edita en PostgreSQL, sincroniza el documento en MeiliSearch en la misma operación de servicio o mediante un hook.

```rust
pub async fn create_listing(pool: &PgPool, meili: &Index, data: Listing) -> Result<(), AppError> {
    // 1. Insertar en DB
    let new_listing = db::insert_listing(pool, &data).await?;
    
    // 2. Sincronizar MeiliSearch
    meili.add_documents(&[new_listing], Some("id")).await?;
    
    Ok(())
}
```

### 5. Sincronización al Eliminar
Incluso si el anuncio se marca como `deleted` (logical delete), debe eliminarse o filtrarse de MeiliSearch.

```rust
pub async fn delete_listing(pool: &PgPool, meili: &Index, id: Uuid) -> Result<(), AppError> {
    // DB delete
    db::delete(pool, id).await?;
    
    // Meili delete
    meili.delete_document(id.to_string()).await?;
    
    Ok(())
}
```

### 6. Query de Búsqueda con Filtros Combinados
Construye búsquedas que combinen texto libre con filtros de categoría, precio y geolocalización.

```rust
let search_result = index.search()
    .with_query("bicicleta")
    .with_filter("price < 200 AND category = 'deportes' AND status = 'active'")
    .with_limit(20)
    .with_offset(0)
    .execute::<Listing>()
    .await?;
```

### 7. Paginación Eficiente
No uses paginación por saltos grandes si puedes usar `limit` y `offset`. Nebripop prefiere scroll infinito o carga por lotes de 20.

```rust
pub struct SearchParams {
    pub q: Option<String>,
    pub page: usize,
    pub limit: usize,
}

// offset = page * limit
```

### 8. Fallback a SQL (Resiliencia)
Si el cliente de MeiliSearch devuelve un error (timeout, servicio caído), la aplicación debe redirigir la consulta a PostgreSQL usando `ILIKE`.

```rust
async fn search_listings(meili: &Index, pool: &PgPool, q: &str) -> Vec<Listing> {
    match meili.search().with_query(q).execute().await {
        Ok(res) => res.hits.into_iter().map(|h| h.result).collect(),
        Err(_) => {
            // Fallback a SQL
            sqlx::query_as!(Listing, 
                "SELECT * FROM listings WHERE title ILIKE $1 AND status = 'active'",
                format!("%{}%", q)
            )
            .fetch_all(pool).await.unwrap_or_default()
        }
    }
}
```

### 9. Indexación Masiva (Bulk Load)
Para despliegues iniciales o recuperaciones, implementa una tarea que indexe todos los anuncios activos de la base de datos.

```rust
pub async fn reindex_all(pool: &PgPool, meili: &Index) -> Result<(), AppError> {
    let listings = sqlx::query_as!(Listing, "SELECT * FROM listings WHERE status = 'active'")
        .fetch_all(pool).await?;
    
    meili.add_documents(&listings, Some("id")).await?;
    Ok(())
}
```

### 10. Ordenación por Relevancia vs Atributos
Por defecto, MeiliSearch ordena por relevancia. Permite al usuario alternar a "Más recientes" o "Precio más bajo".

```rust
let mut search = index.search();
if let Some(sort) = params.sort {
    // sort: ["price:asc"] o ["created_at:desc"]
    search.with_sort(&[sort]);
}
```

## Recomendaciones de Desarrollo
- **MeiliSearch API Key**: Usa siempre una "Search Key" en el frontend y una "Admin Key" solo en el backend.
- **Geofilter**: Para filtrar por distancia, usa `_geoRadius(lat, lng, distance_in_meters)`.
- **Typo Tolerance**: Por defecto está activa; no la desactives a menos que el PRD lo exija para categorías exactas.
