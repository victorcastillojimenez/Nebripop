# Skill: Cloudinary Integration for Nebripop

Esta skill define el estándar para la gestión de imágenes en Nebripop, asegurando una integración robusta entre Axum, Cloudinary y PostgreSQL, con fallback local.

## Contexto
Según el **PRD (Módulo listings)** y el **project-context.md**, las imágenes son críticas para el marketplace. Se utiliza Cloudinary para almacenamiento y transformaciones, MeiliSearch para búsqueda y PostgreSQL para la persistencia del catálogo.

## Reglas y Ejemplos

### 1. Extracción de Multipart en Axum
Utiliza `axum::extract::Multipart` para recibir archivos. Siempre procesa los campos de forma asíncrona para evitar bloqueos.

```rust
use axum::extract::Multipart;

pub async fn upload_handler(mut multipart: Multipart) -> Result<impl IntoResponse, AppError> {
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_string();
        let content_type = field.content_type().unwrap_or_default().to_string();
        let data = field.bytes().await?;
        
        if name == "image" {
            // Validar y procesar
        }
    }
    Ok(StatusCode::OK)
}
```

### 2. Validación de Archivos y Límites
Nebripop limita a **10 imágenes por anuncio**. Valida el tamaño (máx 5MB) y el tipo MIME antes de cualquier procesamiento externo.

```rust
const MAX_UPLOAD_SIZE: usize = 5 * 1024 * 1024; // 5MB
const ALLOWED_MIME: [&str; 3] = ["image/jpeg", "image/png", "image/webp"];

fn validate_image(data: &[u8], mime: &str) -> Result<(), AppError> {
    if data.len() > MAX_UPLOAD_SIZE {
        return Err(AppError::BadRequest("Imagen demasiado grande".into()));
    }
    if !ALLOWED_MIME.contains(&mime) {
        return Err(AppError::BadRequest("Formato no permitido".into()));
    }
    Ok(())
}
```

### 3. Transformaciones Automáticas
No almacenes la URL cruda si puedes aplicar transformaciones en la URL de entrega. Nebripop requiere **800x600**, **formato WebP** y calidad automática.

```rust
/// Transforma una URL de Cloudinary para optimizarla
fn get_optimized_url(original_url: &str) -> String {
    // Inserta parámetros de transformación tras /upload/
    original_url.replace(
        "/upload/", 
        "/upload/c_fill,g_auto,w_800,h_600,f_webp,q_auto/"
    )
}
```

### 4. Persistencia en PostgreSQL
Almacena la URL resultante en la tabla `listing_images`. Asegura el orden mediante el campo `position`.

```rust
pub async fn save_image_metadata(
    pool: &PgPool, 
    listing_id: Uuid, 
    url: String, 
    pos: i16
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO listing_images (id, listing_id, image_url, position) 
         VALUES ($1, $2, $3, $4)",
        Uuid::new_v4(), listing_id, url, pos
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

### 5. Estrategia de Fallback Local
Si las credenciales de Cloudinary no están presentes o el servicio falla, almacena físicamente en `/static/uploads/`.

```rust
async fn store_image(data: &[u8], filename: &str) -> Result<String, AppError> {
    if let Ok(cloudinary_url) = std::env::var("CLOUDINARY_URL") {
        // Lógica de upload a Cloudinary
        // ...
    } else {
        // Fallback local
        let path = format!("static/uploads/{}", filename);
        tokio::fs::write(&path, data).await?;
        Ok(format!("/static/uploads/{}", filename))
    }
}
```

### 6. Eliminación de Recursos
Al borrar un anuncio o una imagen, se debe liberar espacio en Cloudinary usando el `public_id`.

```rust
async fn delete_from_cloudinary(image_url: &str) -> Result<(), AppError> {
    if image_url.contains("res.cloudinary.com") {
        let public_id = extract_public_id(image_url);
        cloudinary_client::destroy(&public_id).await?;
    }
    Ok(())
}
```

### 7. URLs Firmadas para Privacidad
Para imágenes que no deben ser públicas (ej: documentos de verificación), usa URLs firmadas con expiración.

```rust
fn generate_signed_url(public_id: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    let expiry = now + 3600; // 1 hora
    // Cloudinary SDK generará la firma HMAC
    cloudinary.sign_url(public_id).expires_at(expiry).finish()
}
```

### 8. Renderizado en Askama Templates
Utiliza las URLs almacenadas directamente en los templates. Asegúrate de manejar el placeholder si no hay imágenes.

```html
<!-- listing_detail.html -->
<div class="grid grid-cols-2 gap-4">
    {% for img in listing.images %}
    <img src="{{ img.image_url }}" 
         alt="{{ listing.title }}" 
         class="w-full h-64 object-cover rounded-xl shadow-lg hover:scale-105 transition-transform">
    {% empty %}
    <img src="/static/img/no-image.webp" alt="No image" class="opacity-50">
    {% endfor %}
</div>
```

## Recomendaciones de Desarrollo
- **Async everywhere**: El upload es I/O intensivo; nunca bloquees el worker de Tokio.
- **Background Jobs**: Considera mover la eliminación de imágenes a una cola (sidekiq/sqlx-mq) para no retrasar la respuesta al usuario.
- **Placeholders**: Genera una imagen por defecto usando `generate_image` para el desarrollo.
