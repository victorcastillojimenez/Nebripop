---
name: axum-best-practices
description: Directrices de arquitectura, mejores prácticas y patrones de codificación para el backend de Nebripop desarrollado con Rust, Axum, SQLx y Tokio. Utiliza esta skill siempre que vayas a escribir o modificar handlers, routers, extractores, middleware, estados compartidos o gestores de errores en el backend.
---

# Axum Best Practices — Nebripop Backend

Esta skill define la arquitectura oficial, los estándares de diseño de API y los patrones de implementación en Rust y Axum para el backend del marketplace **Nebripop**. El objetivo es garantizar un código robusto, sin panics, altamente performante (P95 < 200ms) y alineado con la arquitectura hexagonal por crates establecida en el proyecto.

---

## 1. Estructura de Routers por Módulo

Para mantener la escalabilidad y modularidad en una arquitectura hexagonal, el enrutamiento se descentraliza en cada crate/módulo de dominio y se unifica en el orquestador principal (`api`).

### Organización del Routing

1. **Crates de Dominio (`users`, `listings`, `chat`, `payments`, `search`)**: Cada crate expone una función de enrutamiento que retorna un `Router<AppState>`.
2. **Crate Orquestador (`api`)**: Importa los routers de cada módulo, los compone usando `.nest` o `.merge` y les asocia el estado global `AppState` y middlewares transversales.

```
Nebripop Workspace
├── crates/
│   ├── api/          # Orquestador (Axum + Servidor)
│   ├── users/        # Rutas de usuarios y auth
│   ├── listings/     # Rutas de anuncios e imágenes
│   ├── chat/         # Rutas de mensajería y WebSockets
│   ├── payments/     # Rutas de Stripe y transacciones
│   └── search/       # Rutas de MeiliSearch y filtrado
```

### Composición en el Orquestador (`crates/api/src/main.rs`)

```rust
use axum::{Router, routing::get};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

// Importar los routers de los crates correspondientes
use users::router::users_router;
use listings::router::listings_router;
use chat::router::chat_router;
use payments::router::payments_router;
use search::router::search_router;

#[tokio::main]
async fn main() {
    // 1. Inicializar Estado Global
    let state = AppState::new().await;

    // 2. Componer e Integrar los routers de cada módulo
    let api_routes = Router::new()
        .nest("/users", users_router())
        .nest("/listings", listings_router())
        .nest("/chat", chat_router())
        .nest("/payments", payments_router())
        .nest("/search", search_router());

    // 3. Aplicar Middlewares globales y Estado
    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", api_routes)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive()) // Configuración específica en Prod
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}
```

---

## 2. Estado Compartido (`AppState`)

El estado compartido debe centralizar todas las dependencias y clientes externos de Nebripop (PostgreSQL via SQLx, Stripe, Cloudinary, MeiliSearch).

### Estructura de `AppState`

Para que un struct pueda utilizarse como State en Axum 0.7, debe implementar `Clone`. La forma óptima es envolver los clientes pesados o no clonables en `std::sync::Arc`.

```rust
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub stripe_client: Arc<stripe::Client>,
    pub cloudinary_config: Arc<CloudinaryConfig>,
    pub meilisearch_client: Arc<meilisearch_sdk::client::Client>,
    pub jwt_secret: String,
}

impl AppState {
    pub async fn new() -> Self {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let db = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to PostgreSQL");

        let stripe_key = std::env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY must be set");
        let stripe_client = Arc::new(stripe::Client::new(&stripe_key));

        let cloudinary_config = Arc::new(CloudinaryConfig::from_env());
        
        let meili_url = std::env::var("MEILI_URL").unwrap_or_else(|_| "http://localhost:7700".to_string());
        let meili_key = std::env::var("MEILI_MASTER_KEY").expect("MEILI_MASTER_KEY must be set");
        let meilisearch_client = Arc::new(meilisearch_sdk::client::Client::new(meili_url, Some(meili_key)).unwrap());

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "super_secret_key_change_me".to_string());

        Self {
            db,
            stripe_client,
            cloudinary_config,
            meilisearch_client,
            jwt_secret,
        }
    }
}
```

---

## 3. Patrón Exacto de un Handler

Todos los handlers de Nebripop deben seguir estrictamente el mismo flujo secuencial para garantizar la coherencia arquitectónica y la robustez del sistema.

### El Pipeline del Handler:
```
[Request] ──> 1. Extractor ──> 2. Validación ──> 3. Usecase (Domain) ──> 4. Response ──> [Client]
```

1. **Extractor**: Extrae datos del request (State, Path, Query, Json, Claims).
2. **Validación**: Valida los datos deserializados usando la librería `validator`.
3. **Usecase**: Delega la lógica de negocio al caso de uso correspondiente (Arquitectura Hexagonal). El handler nunca contiene lógica de negocio directa ni queries SQL directas.
4. **Response**: Mapea el resultado del caso de uso a una respuesta HTTP estructurada con el código de estado adecuado (ej: `201 Created` para inserciones).

### Ejemplo de Implementación del Patrón

```rust
use axum::{
    extract::{State, Path},
    Json,
    http::StatusCode,
};
use validator::Validate;
use serde::Deserialize;

// 1. DTO de entrada con anotaciones de validación
#[derive(Deserialize, Validate)]
pub struct CreateListingDto {
    #[validate(length(min = 3, max = 150, message = "El título debe tener entre 3 y 150 caracteres"))]
    pub title: String,
    #[validate(length(min = 10, message = "La descripción debe tener al menos 10 caracteres"))]
    pub description: String,
    #[validate(range(min = 0.01, message = "El precio debe ser mayor a 0"))]
    pub price: f64,
    pub category: String,
    pub condition: String, // "new", "like_new", "used"
    pub location_lat: f64,
    pub location_lon: f64,
    pub city: String,
}

// 2. Definición del Handler
pub async fn create_listing(
    State(state): State<AppState>,             // 1. Extractor (State)
    claims: Claims,                           // 1. Extractor (JWT Auth)
    Json(payload): Json<CreateListingDto>,    // 1. Extractor (Body Json)
) -> Result<(StatusCode, Json<ListingResponse>), AppError> {
    
    // 2. Validación de inputs
    payload.validate().map_err(AppError::ValidationError)?;

    // Mapeo del DTO al comando de dominio
    let command = CreateListingCommand {
        seller_id: claims.sub, // ID extraído del JWT
        title: payload.title,
        description: payload.description,
        price: payload.price,
        category: payload.category,
        condition: payload.condition,
        location_lat: payload.location_lat,
        location_lon: payload.location_lon,
        city: payload.city,
    };

    // 3. Invocación de la capa de negocio (Usecase / Service)
    let listing = listings::usecases::create_listing_usecase(command, &state.db).await?;

    // Mapeo a DTO de salida
    let response = ListingResponse::from_domain(listing);

    // 4. Response con código correcto (201 Created)
    Ok((StatusCode::CREATED, Json(response)))
}
```

---

## 4. Estructura de Extractores (Extractors)

Axum utiliza extractores para parsear elementos de la petición HTTP. Es crucial estructurarlos y ordenarlos de forma adecuada.

### Regla de Oro del Orden de Extractores en Axum
> [!IMPORTANT]
> Axum requiere que los extractores que consumen el cuerpo de la petición (como `Json<T>` o `Multipart`) se declaren **siempre en último lugar** en la firma del handler. Si pones un extractor como `Path` o `State` después de `Json`, el código no compilará o fallará en tiempo de ejecución.

### 1. Extractor de Ruta (`Path`)
Se utiliza para capturar variables dinámicas en la URL (ej. `/listings/:id`).
```rust
pub async fn get_listing(
    Path(listing_id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ListingResponse>, AppError> { ... }
```

### 2. Extractor de Parámetros de Consulta (`Query`)
Se utiliza para filtros opcionales, ordenación y paginación (típico en `/search` o `/listings`).
```rust
#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub distance_km: Option<f64>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

pub async fn search_listings(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<ListingResponse>>, AppError> { ... }
```

### 3. Extractor de Autenticación Personalizado (`Claims` JWT)
Para evitar extraer manualmente las cabeceras `Authorization` en cada handler, implementamos un extractor personalizado para el struct `Claims`.

```rust
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};
use jsonwebtoken::{decoding_key::DecodingKey, Validation};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: uuid::Uuid, // User ID
    pub role: String,    // "user" o "admin"
    pub exp: usize,      // Expiración timestamp
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
    AppState: axum::extract::FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // 1. Obtener la cabecera Authorization
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(AppError::Unauthorized("Token faltante".to_string()))?;

        // 2. Validar que empiece con "Bearer "
        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized("Formato de token inválido".to_string()));
        }

        let token = &auth_header[7..];

        // 3. Extraer el jwt_secret del estado global a través de FromRef
        let app_state = AppState::from_ref(state);

        // 4. Decodificar y validar el token
        let token_data = jsonwebtoken::decode::<Claims>(
            token,
            &DecodingKey::from_secret(app_state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized("Token inválido o expirado".to_string()))?;

        Ok(token_data.claims)
    }
}
```

### 4. Extractor de Archivos Subidos (`Multipart`)
Se usa en la creación/edición de anuncios para subir imágenes a Cloudinary. Se debe limitar el tamaño máximo de la request para evitar DoS (Denial of Service).

```rust
use axum::extract::Multipart;

pub async fn upload_listing_image(
    State(state): State<AppState>,
    claims: Claims,
    Path(listing_id): Path<uuid::Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ImageUploadResponse>, AppError> {
    // 1. Verificar propiedad del anuncio primero
    listings::usecases::verify_listing_ownership(listing_id, claims.sub, &state.db).await?;

    let mut image_data = None;

    // 2. Iterar sobre las partes del multipart de forma asíncrona
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "image" {
            let data = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            image_data = Some(data);
            break;
        }
    }

    let bytes = image_data.ok_or_else(|| AppError::BadRequest("Campo 'image' faltante".to_string()))?;

    // 3. Subir a Cloudinary delegando en la capa correspondiente
    let image_url = services::cloudinary::upload_to_cloudinary(&bytes, &state.cloudinary_config).await?;

    // 4. Registrar en base de datos
    let saved_image = listings::usecases::save_listing_image(listing_id, &image_url, &state.db).await?;

    Ok(Json(ImageUploadResponse::from_domain(saved_image)))
}
```

---

## 5. Middlewares Requeridos

Nebripop requiere configurar middlewares transversales para garantizar seguridad, accesibilidad CORS y observabilidad de la API.

### 1. CORS Middleware (Cross-Origin Resource Sharing)
Para permitir que el cliente (Askama SSR renderizado por el servidor + JavaScript vanilla) interactúe correctamente con el backend.
```rust
use tower_http::cors::{CorsLayer, Any};
use axum::http::Method;

pub fn cors_layer() -> CorsLayer {
    // En producción, restringir al dominio específico del frontend de Nebripop.
    let is_prod = std::env::var("APP_ENV").unwrap_or_default() == "production";
    
    if is_prod {
        let frontend_url = std::env::var("FRONTEND_URL").expect("FRONTEND_URL must be set in production");
        CorsLayer::new()
            .allow_origin(frontend_url.parse::<axum::http::HeaderValue>().unwrap())
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers(Any)
    } else {
        CorsLayer::permissive() // Para desarrollo local
    }
}
```

### 2. Logging y Trazabilidad (Tower-HTTP + Tracing)
Para monitorizar el rendimiento y registrar las llamadas a la API (indispensable para cumplir el NFR de latencia P95 < 200ms).
```rust
use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse};
use tracing::Level;

pub fn tracing_layer() -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(tower_http::LatencyUnit::Millis),
        )
}
```

---

## 6. Manejo de Errores HTTP con Respuesta Unificada

Nebripop **nunca** debe retornar respuestas en texto plano en caso de fallo, ni filtrar logs internos de la base de datos al cliente. Todos los errores deben serializarse en JSON con una estructura estandarizada.

### Formato Unificado de Error JSON
```json
{
  "error": "ERR_CODE",
  "message": "Mensaje legible para el usuario final",
  "details": null
}
```

### Definición del Tipo Unificado `AppError`

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Error de validación: {0}")]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("No autorizado: {0}")]
    Unauthorized(String),

    #[error("Acceso denegado: {0}")]
    Forbidden(String),

    #[error("Recurso no encontrado: {0}")]
    NotFound(String),

    #[error("Petición incorrecta: {0}")]
    BadRequest(String),

    #[error("Error de base de datos interno")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Error de pasarela de pago (Stripe): {0}")]
    StripeError(String),

    #[error("Error interno del servidor")]
    Internal(String),
}

// Estructura que recibirá el cliente frontend
#[derive(Serialize)]
struct ErrorResponseBody {
    error: &'static str,
    message: String,
    details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message, details) = match self {
            AppError::ValidationError(errs) => {
                let details = serde_json::to_value(&errs).ok();
                (
                    StatusCode::BAD_REQUEST,
                    "VALIDATION_ERROR",
                    "Los datos proporcionados no son válidos".to_string(),
                    details,
                )
            }
            AppError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                msg,
                None,
            ),
            AppError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                "FORBIDDEN",
                msg,
                None,
            ),
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                msg,
                None,
            ),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                msg,
                None,
            ),
            AppError::DatabaseError(err) => {
                // Hacemos log del error real en el servidor de forma segura
                tracing::error!("DATABASE ERROR: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_SERVER_ERROR",
                    "Ha ocurrido un error interno de base de datos".to_string(),
                    None,
                )
            }
            AppError::StripeError(msg) => {
                tracing::error!("STRIPE INTEGRATION ERROR: {}", msg);
                (
                    StatusCode::BAD_GATEWAY,
                    "PAYMENT_GATEWAY_ERROR",
                    format!("Error al procesar el pago: {}", msg),
                    None,
                )
            }
            AppError::Internal(err_details) => {
                tracing::error!("INTERNAL SERVER ERROR: {}", err_details);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_SERVER_ERROR",
                    "Ha ocurrido un error inesperado en el servidor".to_string(),
                    None,
                )
            }
        };

        let body = Json(ErrorResponseBody {
            error: error_code,
            message,
            details,
        });

        (status, body).into_response()
    }
}
```

---

## 7. Patrones Correctos vs. Incorrectos (Ejemplos Comparativos)

### A. Firma del Handler (Orden de los Extractores)

❌ **Incorrecto (No compila en Axum 0.7 porque el body extractor no está al final)**
```rust
// ERROR: Json consume el cuerpo de la petición. La cabecera (Claims) o el State no se extraerán
pub async fn update_listing(
    State(state): State<AppState>,
    Json(payload): Json<UpdateListingDto>, 
    claims: Claims, // Extractor declarado después del body
    Path(id): Path<uuid::Uuid>, // Extractor declarado después del body
) -> Result<StatusCode, AppError> {
    // ...
}
```

Basicamente el compilador de Rust arrojará un error ininteligible relacionado con `FromRequest` e `IntoResponse`.

✅ **Correcto (Los extractores de metadatos van primero; el cuerpo va al final)**
```rust
pub async fn update_listing(
    State(state): State<AppState>,             // 1. Estado
    Path(id): Path<uuid::Uuid>,                 // 2. Path params
    claims: Claims,                             // 3. Claims (Cabeceras)
    Json(payload): Json<UpdateListingDto>,      // 4. BODY (Json siempre al final)
) -> Result<StatusCode, AppError> {
    // ...
}
```

---

### B. Ejecución de Lógica y Queries en Handlers

❌ **Incorrecto (Lógica de negocio y queries acopladas en el Handler. Inviable para testing y viola la arquitectura hexagonal)**
```rust
pub async fn buy_listing_handler(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<BuyRequest>,
) -> Result<StatusCode, AppError> {
    // Queries e inyecciones directas en el controlador HTTP
    let listing = sqlx::query!("SELECT seller_id, price, status FROM listings WHERE id = $1", payload.listing_id)
        .fetch_one(&state.db)
        .await
        .map_err(AppError::DatabaseError)?;

    if listing.status != "active" {
        return Err(AppError::BadRequest("El anuncio ya no está activo".to_string()));
    }

    // Creación de sesión Stripe en el handler
    let session = stripe::CheckoutSession::create(
        &state.stripe_client,
        stripe::CreateCheckoutSession::new()
            // ... parametrización masiva de Stripe ...
    ).await.map_err(|e| AppError::StripeError(e.to_string()))?;

    Ok(StatusCode::OK)
}
```

✅ **Correcto (El Handler delega en un Usecase y mapea tipos de dominio)**
```rust
pub async fn buy_listing_handler(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<BuyRequest>,
) -> Result<Json<CheckoutSessionResponse>, AppError> {
    // 1. Mapear dto a un comando limpio de dominio
    let command = PurchaseCommand {
        listing_id: payload.listing_id,
        buyer_id: claims.sub,
    };

    // 2. Invocar caso de uso (encapsula lógica de base de datos y llamada a Stripe)
    let checkout_session = payments::usecases::process_purchase_usecase(
        command,
        &state.db,
        &state.stripe_client,
    ).await?;

    // 3. Mapear a respuesta DTO
    Ok(Json(CheckoutSessionResponse::from_domain(checkout_session)))
}
```

---

### C. Mutaciones en Múltiples Tablas (Atomacidad)

Si creamos un anuncio que requiere guardar imágenes en `listing_images` o al finalizar una transacción donde se cambia el estado del anuncio a `sold` y se crea una `transaction`, debemos garantizar que si una de las operaciones falla, la base de datos no quede corrupta.

❌ **Incorrecto (Sin transacciones; si la segunda consulta falla, la BD queda inconsistente)**
```rust
pub async fn save_transaction_and_sold_status(
    listing_id: uuid::Uuid,
    buyer_id: uuid::Uuid,
    amount: f64,
    db: &PgPool,
) -> Result<(), AppError> {
    // 1. Cambiar estado a vendido
    sqlx::query!("UPDATE listings SET status = 'sold' WHERE id = $1", listing_id)
        .execute(db)
        .await?;

    // <-- Si el servidor se apaga aquí, el producto queda marcado como vendido pero sin transacción asociada!

    // 2. Crear transacción
    sqlx::query!("INSERT INTO transactions (id, listing_id, buyer_id, amount) VALUES ($1, $2, $3, $4)",
        uuid::Uuid::new_v4(), listing_id, buyer_id, amount)
        .execute(db)
        .await?;

    Ok(())
}
```

✅ **Correcto (Uso de base de datos transaccional con SQLx)**
```rust
pub async fn save_transaction_and_sold_status(
    listing_id: uuid::Uuid,
    buyer_id: uuid::Uuid,
    amount: f64,
    db: &PgPool,
) -> Result<(), AppError> {
    // 1. Iniciar la transacción
    let mut tx = db.begin().await.map_err(AppError::DatabaseError)?;

    // 2. Ejecutar primera query dentro de la transacción
    sqlx::query!("UPDATE listings SET status = 'sold' WHERE id = $1", listing_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

    // 3. Ejecutar segunda query
    sqlx::query!("INSERT INTO transactions (id, listing_id, buyer_id, amount) VALUES ($1, $2, $3, $4)",
        uuid::Uuid::new_v4(), listing_id, buyer_id, amount)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

    // 4. Confirmar transacción
    tx.commit().await.map_err(AppError::DatabaseError)?;

    Ok(())
}
```

---

## 8. Las 12 Reglas Críticas de Axum para Nebripop

Para garantizar el cumplimiento de los Requisitos No Funcionales (NFRs) de Nebripop, cualquier generación de backend debe adherirse a estas 12 reglas de oro:

1. **Orden Obligatorio de Extractores**: Coloca siempre los extractores de metadatos (`State`, `Path`, `Query`, `Claims`) primero en la firma del handler, y los extractores de cuerpo (`Json`, `Multipart`) estrictamente en último lugar.
2. **Cero Panics (`unwrap`/`expect`)**: Nunca utilices `unwrap()` o `expect()` dentro de handlers, casos de uso o adaptadores. Utiliza propagación de errores (`?`) y mapeos seguros con `.map_err()` hacia un `AppError`.
3. **Validación Explícita en la Frontera**: Todo DTO de entrada en un endpoint de escritura (`POST`, `PUT`) debe validar sus campos anotándolos con la macro `#[derive(Validate)]` de la librería `validator` y ejecutando `payload.validate()?` antes de iniciar cualquier lógica.
4. **Filtros Geoespaciales en Base de Datos**: Las consultas de proximidad (distancia en km) de anuncios deben calcularse a nivel de base de datos usando la fórmula de Haversine dentro de la query SQL. **Nunca** cargues todos los anuncios a memoria en Rust para filtrarlos localmente.
5. **No Exponer Detalles de la Base de Datos**: Nunca retornes los structs mapeados de base de datos directamente al cliente. Genera siempre DTOs de salida específicos (`ListingResponse`, `UserResponse`) para asegurar que contraseñas, hashes y datos técnicos queden ocultos.
6. **Manejo Atomico con Transacciones**: Cualquier lógica que realice múltiples escrituras de base de datos en cadena debe ejecutarse en una transacción controlada (`db.begin().await?`) para evitar estados corruptos del sistema.
7. **Límite Estricto en Subida de Archivos (Multipart)**: En la subida de imágenes para anuncios (Cloudinary), aplica siempre límites de tamaño en el handler y lee los chunks de bytes asíncronamente mediante buffers secuenciales para evitar fugas de memoria y ataques DoS.
8. **Validación Rigurosa de Webhooks de Stripe**: El endpoint que recibe notificaciones de Stripe debe validar de manera estricta la firma `Stripe-Signature` utilizando la clave de webhook oficial (`STRIPE_WEBHOOK_SECRET`). Nunca proceses un pago basándote en un payload JSON no autenticado.
9. **CORS Dinámico por Entorno**: En desarrollo local (`APP_ENV=development`), permite orígenes comodín (`*`) para agilizar pruebas. En producción (`APP_ENV=production`), restringe CORS exclusivamente al dominio de producción de Nebripop.
10. **Aislamiento en Casos de Uso (Hexagonal)**: Los handlers deben actuar estrictamente como controladores de transporte de red HTTP. La lógica de negocio real debe residir dentro de `usecases` o `domain_services`, los cuales no deben importar tipos de `axum` ni de `tower-http`.
11. **Errores Ocultos en el Servidor (Security)**: Los errores de base de datos (`sqlx::Error`) o de red deben escribirse en los logs del sistema (`tracing::error!`) para depuración interna, pero al cliente externo se le debe retornar un mensaje genérico unificado de tipo `INTERNAL_SERVER_ERROR`.
12. **Inyección Limpia de Dependencias**: Pasa los componentes compartidos (PgPool, Clientes de APIs externas) a los casos de uso a través de referencias simples (`&state.db`, `&state.stripe_client`) extraídas en el handler. Evita inyectar el struct masivo `AppState` completo en capas profundas del dominio.
