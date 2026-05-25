---
name: error-handling-rust
description: Directrices de arquitectura, mejores prácticas y patrones de codificación para el manejo de errores robusto, tipado y seguro en el backend de Rust + Axum de Nebripop. Utiliza esta skill siempre que vayas a escribir, refactorizar o revisar flujos de error, propagación, mapeos HTTP o logging con tracing.
---

# Error Handling & Propagation in Rust — Nebripop

Esta skill define las directrices y estándares para estructurar, capturar, propagar y formatear errores en el backend de **Nebripop**. El objetivo es garantizar un sistema robusto libre de panics en producción, con errores de dominio fuertemente tipados mediante **`thiserror`**, y una separación rigurosa entre la información técnica interna (que debe registrarse de forma segura en los logs) y las respuestas amigables entregadas al cliente web en formato JSON.

---

## 1. Errores de Dominio por Módulo (`thiserror`)

Para estructurar los errores dentro del dominio de Nebripop, utilizaremos la macro `#[derive(thiserror::Error)]` para construir enums específicos por cada módulo funcional (`users`, `listings`, `payments`, `chat`). Esto nos permite asociar mensajes explicativos en tiempo de compilación y estructurar de manera cohesiva el negocio.

### Ejemplo de Definición por Módulo

```rust
// crates/listings/src/errors.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ListingError {
    #[error("Anuncio con ID {0} no encontrado")]
    NotFound(uuid::Uuid),

    #[error("El anuncio ya ha sido marcado como vendido")]
    AlreadySold(uuid::Uuid),

    #[error("El comprador no puede ser el mismo que el vendedor del anuncio")]
    SelfPurchase,

    #[error("Error de base de datos interno: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Fallo de infraestructura externa: {0}")]
    Infrastructure(String),
}
```

---

## 2. Ámbito de Aplicación: `thiserror` vs `anyhow`

Para asegurar la tipicidad del compilador, delimitamos de manera estricta el uso de ambas librerías en el workspace de Nebripop.

* **`thiserror`**: Obligatorio en todos los módulos de lógica de negocio (domain crates), controladores (handlers) y adaptadores de base de datos/infraestructura. Permite modelar tipos concretos que el compilador exige tratar de forma explícita.
* **`anyhow`**: Permitido **exclusivamente** en el punto de entrada binario (`crates/api/src/main.rs`) y en la suite de pruebas unitarias o de integración (`tests/`). Sirve para capturar errores de configuración o fallos globales en el arranque de la aplicación.

---

## 3. Propagación Segura y Prohibición de `unwrap()` y `expect()`

> [!WARNING]
> **Está estrictamente prohibido utilizar `.unwrap()` o `.expect()` en la lógica de producción (handlers, usecases, repositorios) del backend de Nebripop. Cualquier llamada a estas funciones es causa directa de rechazo de código.**

En su lugar, se debe utilizar la propagación segura mediante el operador **`?`**, o manejar el flujo explícitamente usando mapeadores como `.ok_or()`, `.ok_or_else()` o bloques `match`.

* **tests/**: Se permite el uso de `.unwrap()` o `.expect("mensaje explicativo del fallo del test")` de forma exclusiva en los tests, ya que allí el pánico es la forma correcta de indicar que un caso de prueba ha fallado.

---

## 4. Conversión de Errores Técnicos a Errores de Dominio

La capa de adaptadores (PostgreSQL/SQLx, Stripe API) debe interceptar las excepciones técnicas y convertirlas de inmediato en errores semánticos del dominio. Esto evita que los detalles internos de base de datos (como tablas o columnas) se filtren a capas superiores.

```rust
// Ejemplo en el repositorio de base de datos
pub async fn find_user_by_id(
    user_id: uuid::Uuid,
    db: &sqlx::PgPool
) -> Result<User, UserError> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_optional(db)
        .await
        // 1. Mapear error de conexión/driver sqlx::Error
        .map_err(UserError::Database)?
        // 2. Mapear ausencia del registro a un error semántico explícito
        .ok_or(UserError::NotFound(user_id))
}
```

---

## 5. Mapeo de Errores de Dominio a Respuestas HTTP (`IntoResponse`)

Para unificar la salida hacia la interfaz de usuario de Nebripop, creamos el enum global `AppError` en la capa del orquestador web (`api`). Este enum implementa el trait `IntoResponse` de Axum 0.7 para transformar los errores lógicos en respuestas HTTP JSON con el código de estado adecuado.

### Implementación del Enum `AppError` y su `IntoResponse`

```rust
use axum::{
    response::{IntoResponse, Response, Json},
    http::StatusCode,
};
use serde::Serialize;
use tracing;

// Cuerpo JSON unificado entregado al cliente web
#[derive(Serialize)]
pub struct ErrorResponseBody {
    pub success: bool,
    pub error_code: String,
    pub message: String,
}

#[derive(Debug)]
pub enum AppError {
    ValidationError(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    PaymentFailed(String),
    InternalServerError, // No expone detalles técnicos
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "ALREADY_EXISTS", msg),
            AppError::PaymentFailed(msg) => (StatusCode::PAYMENT_REQUIRED, "PAYMENT_FAILED", msg),
            AppError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_SERVER_ERROR",
                "Ha ocurrido un error interno del servidor. Por favor, inténtelo de nuevo más tarde.".to_string(),
            ),
        };

        let body = Json(ErrorResponseBody {
            success: false,
            error_code: error_code.to_string(),
            message,
        });

        (status, body).into_response()
    }
}
```

### Conversión Asíncrona en los Handlers

```rust
// Handler de Axum
pub async fn get_listing_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ListingDto>, AppError> {
    // 1. Invocar caso de uso y mapear el error específico de dominio a AppError
    let listing = listings::usecases::get_listing_by_id(id, &state.db)
        .await
        .map_err(|err| match err {
            ListingError::NotFound(uid) => AppError::NotFound(format!("El anuncio con ID {} no existe.", uid)),
            ListingError::AlreadySold(_) => AppError::Conflict("El anuncio ya está vendido.".to_string()),
            ListingError::Database(db_err) => {
                // Registrar de forma segura el fallo técnico en los logs del servidor
                tracing::error!("Fallo en base de datos al buscar anuncio {}: {:?}", id, db_err);
                // Retornar error genérico amigable al usuario (Sin filtrar SQL)
                AppError::InternalServerError
            }
            _ => AppError::InternalServerError,
        })?;

    Ok(Json(ListingDto::from_domain(listing)))
}
```

---

## 6. Errores Críticos del PRD

Mapeo de correspondencias recomendadas para los errores específicos del PRD:

| Escenario del PRD | Error de Dominio | Código de Respuesta HTTP |
|-------------------|------------------|--------------------------|
| **Ficha no encontrada (`US-05`)** | `ListingError::NotFound` | `404 Not Found` |
| **Token expirado / Mal firmado** | `UserError::TokenExpired` | `401 Unauthorized` |
| **Pago fallido Stripe (`US-18`)** | `PaymentError::StripeDeclined` | `402 Payment Required` (o `400`) |
| **Email duplicado (`US-01`)** | `UserError::EmailDuplicated` | `409 Conflict` |
| **Autocompra prohibida (`US-17`)** | `ListingError::SelfPurchase` | `400 Bad Request` |

---

## 7. Logging Seguro con `tracing` sin Fugas de Datos

Cuando se registra un error mediante las macros de `tracing` (`tracing::error!`, `tracing::warn!`), debemos garantizar que **nunca se escriban en texto plano datos confidenciales o regulados por RGPD o PCI-DSS**.

### Qué datos NO deben loggearse jamás en texto plano:
* Contraseñas en texto plano.
* Hashes de contraseñas (`password_hash`).
* Tokens JWT completos (`Authorization` cabecera completa).
* Números de tarjeta de crédito (PAN) o códigos de seguridad (CVC).

### Ejemplo de Log Seguro
```rust
// ❌ INSECURO: Fuga de credenciales críticas en logs
tracing::error!("Error de login con email {} y password {}", payload.email, payload.password);

// ✅ SEGURO: Registra el email (identificador de negocio) pero oculta cualquier rastro de la contraseña
tracing::warn!(
    target: "auth_events",
    user_email = %payload.email,
    "Intento fallido de inicio de sesión por credenciales incorrectas."
);
```

---

## 8. Patrones Correctos vs. Incorrectos

### A. Gestión de Ausencia de Registros

❌ **Incorrecto (Uso de unwrap/expect expuesto a panics en producción si el ID no existe en la base de datos)**
```rust
pub async fn get_user_bad(id: uuid::Uuid, db: &PgPool) -> User {
    // Si la query retorna None, la aplicación sufrirá un panic crash de inmediato!
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(db)
        .await
        .unwrap() // ¡TOTALMENTE PROHIBIDO!
        .unwrap() // ¡TOTALMENTE PROHIBIDO!
}
```

✅ **Correcto (Mapeo explícito y controlado de la ausencia usando tipos de dominio semánticos)**
```rust
pub async fn get_user_good(id: uuid::Uuid, db: &PgPool) -> Result<User, UserError> {
    let result = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(db)
        .await
        .map_err(UserError::Database)?;

    match result {
        Some(user) => Ok(user),
        None => Err(UserError::NotFound(id)),
    }
}
```

---

### B. Mapeo de Errores de API Externa (Stripe)

❌ **Incorrecto (Propagar errores directos del SDK de Stripe hacia el frontend de Axum expone librerías internas y llaves privadas)**
```rust
pub async fn checkout_handler_bad(
    State(state): State<AppState>,
    Json(payload): Json<ChargeDto>
) -> Result<StatusCode, String> {
    // Retornar String crudo de error es una pésima práctica de Clean Code
    let receipt = stripe::Charge::create(&state.stripe_client, ...)
        .await
        .map_err(|err| err.to_string())?; // Expone información de red, API keys o tablas internas
        
    Ok(StatusCode::OK)
}
```

✅ **Correcto (Capturar el error técnico del SDK, mapear en capa de pagos a un error de dominio y formatear JSON controlado)**
```rust
pub async fn checkout_handler_good(
    State(state): State<AppState>,
    Json(payload): Json<ChargeDto>
) -> Result<StatusCode, AppError> {
    
    // 1. Invocar el caso de uso
    payments::usecases::process_payment_usecase(&payload, &state.db, &state.stripe_client)
        .await
        .map_err(|err| match err {
            PaymentError::StripeDeclined(msg) => AppError::PaymentFailed(msg),
            PaymentError::Database(db_err) => {
                tracing::error!("Error SQLx en pasarela de cobro: {:?}", db_err);
                AppError::InternalServerError
            }
            _ => AppError::InternalServerError,
        })?;

    Ok(StatusCode::OK)
}
```

---

## 9. Las 12 Reglas Críticas de Manejo de Errores para Nebripop

1. **Uso Exclusivo de thiserror**: Define los errores lógicos del dominio en cada módulo crate utilizando el enum derivado con `thiserror`. Está prohibido definir flujos de error con cadenas de texto genéricas (`Result<T, String>`).
2. **Restricción de anyhow**: Limita el uso de `anyhow::Result` o `anyhow::Error` estrictamente al archivo inicializador `main.rs` del orquestador web y a los ficheros dentro de `tests/`.
3. **Cero unwrap/expect en Producción**: Ningún fichero bajo la carpeta `src/` de los crates del workspace (a excepción de los tests) puede contener llamadas a `.unwrap()` o `.expect()`.
4. **Propagación Segura**: Utiliza el operador `?` para delegar y propagar errores a través de la pila de llamadas del backend de forma natural y limpia.
5. **Aislamiento Técnico (SQLx)**: Convierte de inmediato los fallos del driver de base de datos `sqlx::Error` a enums específicos del dominio (`ListingError::Database`) dentro de los adaptadores de persistencia.
6. **Mapeo Unificado HTTP**: Implementa el trait `IntoResponse` en el enum central `AppError` para estandarizar los códigos de respuesta y la estructura del JSON devuelto al navegador.
7. **Ocultamiento de Fallos Internos**: Si ocurre un error técnico de infraestructura (caída de BD, fallo de conexión a Stripe), regístralo de forma interna en los logs y retorna un código `500 Internal Server Error` con el mensaje genérico seguro: *"Ha ocurrido un error interno del servidor"*.
8. **Validación de Parámetros en Entrada**: Los errores de formato detectados en los DTOs (`ValidationError`) deben retornar inmediatamente un estado `400 Bad Request` detallando qué campos incumplen los requisitos.
9. **Logging Libre de Datos Sensibles**: Está prohibido escribir contraseñas en texto plano, hashes de claves (`password_hash`), números de tarjeta de crédito (CVC/PAN) o JWTs completos en los mensajes del logger `tracing`.
10. **Especificación de Identificadores**: Todo error de ausencia de registro (ej. `ListingError::NotFound(Uuid)`) debe contener obligatoriamente el identificador del registro no encontrado para facilitar el diagnóstico en los logs de desarrollo.
11. **Manejo Específico de Claves Duplicadas**: Captura de forma explícita los errores de colisión única en registro de usuarios (SQLSTATE `23505`) y mapéalos como `UserError::EmailDuplicated` devolviendo un código `409 Conflict`.
12. **Tests Autodocumentados**: Cuando utilices `.expect()` en la suite de pruebas unitarias o de integración, proporciona un mensaje claro y descriptivo del comportamiento esperado (ej: `.expect("La base de datos de test debería haber devuelto el anuncio activo recién creado")`).
