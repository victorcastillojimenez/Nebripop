# Skill: Rust Axum Handlers for Nebripop

Esta skill define la plantilla maestra para todos los controladores (handlers) de Nebripop. Asegura consistencia en la validación, códigos de estado HTTP y manejo de errores siguiendo la arquitectura hexagonal.

## Contexto
Todos los handlers deben ser asíncronos, retornar `Result<Response, AppError>` y mantener una lógica delegada en la capa de dominio/usecase.

## Reglas y Convenciones

### 1. Convención de Nomenclatura
- **Función**: `handle_[action]_[entity]` (ej: `handle_create_listing`).
- **Input**: `[Action][Entity]Request` (ej: `CreateListingRequest`).
- **Output**: `[Action][Entity]Response` (ej: `CreateListingResponse`).

### 2. Plantilla Maestra de Handler
Sigue siempre este patrón para garantizar trazabilidad y validación.

```rust
pub async fn handle_create_listing(
    auth: AuthUser, // Extractor de JWT
    State(state): State<AppState>,
    Json(payload): Json<CreateListingRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Validación
    payload.validate()?;

    // 2. Ejecución (Llamada al Dominio)
    let listing = state.listing_service.create(auth.user_id, payload.into()).await?;

    // 3. Respuesta
    Ok((StatusCode::CREATED, Json(CreateListingResponse::from(listing))))
}
```

### 3. Códigos de Estado HTTP y Uso
- **200 OK**: Consultas exitosas (GET, PUT parcial).
- **201 Created**: Creación exitosa de recursos (POST).
- **400 Bad Request**: Errores de validación o lógica de negocio inválida.
- **401 Unauthorized**: JWT inválido o ausente.
- **403 Forbidden**: El usuario no es el dueño del recurso (ej: borrar anuncio de otro).
- **404 Not Found**: El recurso solicitado no existe.
- **500 Internal Server Error**: Error de base de datos o fallo del sistema.

### 4. Handler de Subida de Imágenes (Multipart)
Usa esta estructura para procesar metadatos y archivos en la misma petición.

```rust
pub async fn handle_upload_image(
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_string();
        let data = field.bytes().await?;
        // Lógica de procesamiento...
    }
    Ok(StatusCode::OK)
}
```

### 5. Paginación y Filtros (Query Params)
Utiliza un extractor común para manejar `page` y `per_page`.

```rust
#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

pub async fn handle_list_listings(
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params.per_page.unwrap_or(20);
    let offset = params.page.unwrap_or(0) * limit;
    // ...
}
```

### 6. WebSocket para Chat en Tiempo Real
Plantilla para el endpoint de chat bidireccional.

```rust
pub async fn handle_chat_ws(
    ws: WebSocketUpgrade,
    auth: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, auth.user_id, state))
}

async fn handle_socket(mut socket: WebSocket, user_id: Uuid, state: AppState) {
    // Lógica de bucle de mensajes (select! de tokio)
}
```

---

## Ejemplos de Handlers Complejos

### Ejemplo 1: POST /listings (Crear con Imagen)
Combina persistencia en DB con integración externa.

```rust
pub async fn handle_create_listing_full(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let mut listing_data = None;
    let mut images = Vec::new();

    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("data") => listing_data = Some(serde_json::from_slice::<CreateListingDTO>(&field.bytes().await?)?),
            Some("image") => {
                let url = state.image_service.upload(field.bytes().await?).await?;
                images.push(url);
            },
            _ => {}
        }
    }

    let data = listing_data.ok_or(AppError::BadRequest("Missing listing data".into()))?;
    let listing = state.listing_usecase.create(auth.user_id, data, images).await?;
    
    Ok((StatusCode::CREATED, Json(listing)))
}
```

### Ejemplo 2: POST /payments (Iniciar Pago)
Orquestación con Stripe.

```rust
pub async fn handle_initiate_payment(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<PaymentRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Verificar disponibilidad del anuncio
    let listing = state.listing_repo.get_by_id(payload.listing_id).await?;
    if listing.status != "active" { return Err(AppError::Conflict("Not available".into())); }

    // 2. Crear intención en Stripe
    let pi = state.stripe_service.create_intent(listing.price, listing.id).await?;

    // 3. Registrar transacción pendiente en DB
    state.payment_repo.create_pending(auth.user_id, listing.id, pi.id).await?;

    Ok(Json(PaymentResponse { client_secret: pi.client_secret }))
}
```

### Ejemplo 3: GET /search (Búsqueda Avanzada)
Integración con MeiliSearch y filtros.

```rust
pub async fn handle_search(
    Query(params): Query<SearchFilters>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let results = state.search_service.find(
        params.q.as_deref().unwrap_or(""),
        params.category.as_deref(),
        params.max_price,
        params.distance_km
    ).await?;

    Ok(Json(results))
}
```

## Recomendaciones finales
- **Json Serialization**: Todos los modelos de respuesta deben derivar `Serialize`.
- **Validation**: Usa el crate `validator` para reglas comunes (`email`, `length`, etc.).
- **Response wrapping**: No devolver entidades de base de datos directamente; transformar siempre a DTOs para no exponer campos como `password_hash`.
