---
name: security-audit-rust
description: Guía completa y directrices para auditar y securizar el backend de Rust + Axum de Nebripop frente a vulnerabilidades del OWASP Top 10, fugas de secretos y fallos de autenticación/pagos. Úsala siempre que vayas a realizar una revisión de seguridad o escribir código crítico.
---

# Security Audit & Secure Coding Best Practices — Nebripop

Esta skill define la guía completa y el manual operativo de auditoría de seguridad para el backend y frontend de **Nebripop**. Como marketplace de segunda mano C2C que gestiona transacciones económicas, datos personales y mensajería en tiempo real, Nebripop debe cumplir de forma rigurosa con políticas activas para mitigar riesgos del **OWASP Top 10**, inyecciones de código, fugas de secretos, suplantación de identidad y brechas en la pasarela de pagos.

---

## 1. Autenticación y Autorización

### Qué verificar exactamente:
1. **Firma y Algoritmo de JWT**: Comprobar que solo se aceptan algoritmos seguros (`HS256` o `RS256`). Bloquear explícitamente el uso de `None` en la validación del token.
2. **Fortaleza de Argon2id**: Asegurar que las contraseñas se procesan usando Argon2id con parámetros recomendados por OWASP (`memoria >= 19MB (19456 KB)`, `iteraciones >= 2`, `paralelismo >= 1`).
3. **Rate Limiting**: Exigir middleware de control de tasa de peticiones (Rate Limiting) en endpoints críticos de autenticación (`POST /auth/register`, `POST /auth/login`) para mitigar ataques de fuerza bruta y denegación de servicio.
4. **Verificación de Rol en Servidor**: Validar que la autorización y control de acceso (rol `user` vs `admin`) se realiza siempre en el backend (Axum Middleware/Extractor), nunca delegando la confianza únicamente en la visualización del frontend.

### ❌ Código Incorrecto (Vulnerable)
```rust
// ❌ VULNERABLE: Criptografía débil, sin rate limit, validación de JWT sin control estricto de algoritmo
pub fn hash_weak(password: &str) -> String {
    // Uso obsoleto de SHA256 sin salting aleatorio. Muy fácil de revertir mediante tablas arcoíris
    let mut hasher = sha2::Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn decode_jwt_unsafe(token: &str, secret: &str) -> Result<Claims, ()> {
    let mut validation = jsonwebtoken::Validation::default();
    // Permitir "None" es una vulnerabilidad crítica de suplantación de firma!
    validation.algorithms = vec![jsonwebtoken::Algorithm::HS256, jsonwebtoken::Algorithm::None];
    
    jsonwebtoken::decode::<Claims>(token, &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()), &validation)
        .map(|data| data.claims)
        .map_err(|_| ())
}
```

### ✅ Código Seguro (Remediado)
```rust
// ✅ SEGURO: Parámetros OWASP para Argon2id y validación estricta de algoritmo HS256
pub fn hash_secure(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());
    let params = argon2::Params::new(19456, 2, 1, None)?; // OWASP
    let argon2 = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V13, params);
    Ok(argon2.hash_password(password.as_bytes(), &salt)?.to_string())
}

pub fn decode_jwt_secure(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256); // Solo HS256
    validation.validate_exp = true; // Forzar verificación de tiempo
    
    jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &validation
    ).map(|data| data.claims)
}
```

---

## 2. Protección de Datos y Privacidad

### Qué verificar exactamente:
1. **Logging Limpio**: Auditar el uso de macros `tracing::info!/error!/warn!` garantizando que no se logueen contraseñas, emails completos, tokens `Authorization` ni números de tarjeta de crédito.
2. **Variables de Entorno**: Comprobar que secretos de APIs (`STRIPE_SECRET_KEY`, `JWT_SECRET`) nunca estén hardcodeados en el código.
3. **Mapeo de DTOs en Respuestas**: Asegurar que las respuestas de la API mapean los datos a DTOs específicos de salida. Bajo ningún concepto una entidad de base de datos que contenga campos como `password_hash` o `verification_token` debe ser serializada directamente hacia el cliente.

### ❌ Código Incorrecto (Vulnerable)
```rust
// ❌ VULNERABLE: Fuga de datos personales sensibles en logs y exposición de hash de contraseña en la API
#[derive(Serialize)]
pub struct BadUserResponse {
    pub id: uuid::Uuid,
    pub email: String,
    pub password_hash: String, // Fuga crítica de hash en el JSON devuelto!
}

pub async fn login_unsafe(Json(payload): Json<LoginDto>) {
    // Fuga grave de credenciales en texto plano en logs del sistema
    tracing::error!("Intento fallido de login para email: {} con contraseña: {}", payload.email, payload.password);
}
```

### ✅ Código Seguro (Remediado)
```rust
// ✅ SEGURO: Mapeo estricto del DTO de salida libre de campos sensibles
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecureUserResponse {
    pub id: uuid::Uuid,
    pub display_name: String, // Solo datos públicos
}

pub async fn login_safe(Json(payload): Json<LoginDto>) {
    // Se loguea solo el email en un campo estructurado, nunca la contraseña
    tracing::warn!(
        target: "auth_events",
        user_email = %payload.email,
        "Intento fallido de inicio de sesión por credenciales incorrectas."
    );
}
```

---

## 3. Seguridad en Endpoints e Inyecciones

### Qué verificar exactamente:
1. **Prevención de Inyección SQL**: Comprobar que todas las consultas a base de datos mediante SQLx utilicen macros compiladas de paso de parámetros (`query_as!`, `query!`, `bind`). Queda terminantemente prohibido generar consultas SQL concatenando cadenas con la macro `format!`.
2. **Límite Físico de Archivos**: Verificar que el middleware de carga de archivos (imágenes de anuncios) de Axum limite el tamaño máximo a 10MB por archivo para prevenir ataques de denegación de servicio por agotamiento de almacenamiento.
3. **MIME-Type Real**: Comprobar que la validación de archivos subidos analice los bytes mágicos (firma real del archivo) mediante crates como `infer` o `magic`, y no confíe únicamente en la extensión del fichero (ej. cambiar `.exe` a `.jpg`).

### ❌ Código Incorrecto (Vulnerable)
```rust
// ❌ VULNERABLE: Inyección SQL y validación de archivo insegura basada en extensión de texto
pub async fn get_listings_unsafe(category: &str, db: &PgPool) -> Result<Vec<Listing>, sqlx::Error> {
    // FÁCIL INYECCIÓN SQL. Si la categoría tiene un string malicioso, compromete la BD completa
    let raw_query = format!("SELECT * FROM listings WHERE category = '{}'", category);
    sqlx::query_as::<_, Listing>(&raw_query).fetch_all(db).await
}

pub fn check_file_unsafe(filename: &str) -> bool {
    // Vulnerable. Un atacante puede subir un script de PHP o ejecutable renombrado a .png
    filename.ends_with(".png") || filename.ends_with(".jpg")
}
```

### ✅ Código Seguro (Remediado)
```rust
// ✅ SEGURO: Parámetros tipados de SQLx y validación MIME por Magic Bytes
pub async fn get_listings_safe(category: &str, db: &PgPool) -> Result<Vec<Listing>, sqlx::Error> {
    // Parámetros bindeados de forma nativa. Inyección SQL imposible
    sqlx::query_as!(Listing, "SELECT * FROM listings WHERE category = $1", category)
        .fetch_all(db)
        .await
}

pub fn check_file_secure(file_bytes: &[u8]) -> bool {
    // Inspección de bytes mágicos del archivo real
    if let Some(kind) = infer::get(file_bytes) {
        return kind.mime_type() == "image/png" || kind.mime_type() == "image/jpeg";
    }
    false
}
```

---

## 4. Seguridad en Pagos (Stripe Gateway)

### Qué verificar exactamente:
1. **Validación de Firmas Webhook**: Asegurar que el webhook de Stripe valida de forma matemática la firma recibida en la cabecera `Stripe-Signature` utilizando la clave secreta `STRIPE_WEBHOOK_SECRET`.
2. **Cero Datos Financieros en BD**: Confirmar que no se almacena ningún dato confidencial de tarjeta (PAN, CVC) en la base de datos de Nebripop. Solo se guardan los tokens de referencia emitidos por Stripe (`payment_intent_id`, `charge_id`).
3. **Claves de Idempotencia**: Comprobar que la creación de transacciones e intenciones de cobro incluye una clave de idempotencia única (UUID) para evitar cargos dobles al comprador en caso de caídas de red o reintentos del cliente.

### ❌ Código Incorrecto (Vulnerable)
```rust
// ❌ VULNERABLE: Confianza ciega en webhook sin firma y cargos propensos a duplicación
pub async fn webhook_unsafe(
    body: String, // Sin verificar procedencia ni cabecera
) -> StatusCode {
    let event: stripe::Event = serde_json::from_str(&body).unwrap();
    // Procesar evento sin firma. Un atacante puede enviar un JSON falso y marcar anuncios como pagados!
    process_payment_event(event).await;
    StatusCode::OK
}
```

### ✅ Código Seguro (Remediado)
```rust
// ✅ SEGURO: Verificación matemática de firma de Stripe y uso de Clave de Idempotencia
pub async fn webhook_safe(
    headers: axum::http::HeaderMap,
    body: String,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let signature = headers
        .get("Stripe-Signature")
        .and_then(|val| val.to_str().ok())
        .ok_or(AppError::Unauthorized("Firma ausente".to_string()))?;

    // Verificar firma con la librería oficial
    let event = stripe::Webhook::construct_event(&body, signature, &state.stripe_webhook_secret)
        .map_err(|e| AppError::Unauthorized(format!("Firma de Stripe inválida: {}", e)))?;

    process_payment_event(event).await?;
    Ok(StatusCode::OK)
}

pub async fn create_charge_safe(stripe_client: &stripe::Client, amount: i64, listing_id: uuid::Uuid) {
    let idempotency_key = format!("pay-{}", listing_id); // Evita duplicaciones para el mismo anuncio
    
    let mut params = stripe::CreatePaymentIntent::new(amount, stripe::Currency::EUR);
    // ...
    stripe::PaymentIntent::create(stripe_client, params) // Inyectar idempotency key en opciones
}
```

---

## 5. Seguridad en WebSockets (Chat en Tiempo Real)

### Qué verificar exactamente:
1. **Autenticación en el Handshake**: Validar que las conexiones WebSocket exigen el parámetro Query `token` y validan el JWT de forma estricta antes de conceder la elevación del protocolo.
2. **Validación de Pertenencia**: Garantizar que el usuario autenticado del socket solo puede enviar o escuchar mensajes de una sala de chat de la cual es participante legítimo (`buyer_id` o `seller_id` en la BD).
3. **Sanitización de Contenido**: Aplicar sanitización estricta sobre el texto de los mensajes recibidos del chat mediante librerías como `ammonia` para neutralizar ataques XSS almacenados en la base de datos que puedan ejecutarse en el navegador de otros usuarios.
4. **Control de Spam (Message Rate Limiting)**: Limitar la cantidad máxima de mensajes por segundo por canal WebSocket activo para evitar caídas provocadas por scripts automatizados maliciosos.

### ❌ Código Incorrecto (Vulnerable)
```rust
// ❌ VULNERABLE: Sin sanitizar contenido y sin verificar pertenencia
pub async fn process_chat_unsafe(sender_id: Uuid, conversation_id: Uuid, content: &str, db: &PgPool) {
    // Si el contenido contiene `<script>alert('XSS')</script>`, se inyectará en la pantalla del receptor!
    sqlx::query!("INSERT INTO messages (conversation_id, sender_id, content) VALUES ($1, $2, $3)", conversation_id, sender_id, content)
        .execute(db)
        .await
        .ok();
}
```

### ✅ Código Seguro (Remediado)
```rust
// ✅ SEGURO: Sanitización de etiquetas HTML (prevención de XSS) y verificación de miembro
pub async fn process_chat_safe(
    sender_id: Uuid, 
    conversation_id: Uuid, 
    content: &str, 
    db: &PgPool
) -> Result<(), AppError> {
    // 1. Validar pertenencia antes de procesar nada
    let is_member = check_membership(conversation_id, sender_id, db).await?;
    if !is_member {
        return Err(AppError::Forbidden("No perteneces a esta conversación".to_string()));
    }

    // 2. Sanitizar texto para mitigar XSS almacenado
    let sanitized_content = ammonia::clean(content);

    // 3. Persistir cadena de texto segura
    sqlx::query!(
        "INSERT INTO messages (id, conversation_id, sender_id, content, created_at) VALUES ($1, $2, $3, $4, now())",
        uuid::Uuid::new_v4(), conversation_id, sender_id, sanitized_content
    )
    .execute(db)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(())
}
```

---

## 6. Cabeceras HTTP de Seguridad (Security Headers)

### Qué verificar exactamente:
1. **Configuración de CORS**: Bloquear el uso de comodines generales (`*`) en entornos de producción. Declarar explícitamente los orígenes universitarios autorizados de Nebripop.
2. **Content-Security-Policy (CSP)**: Incorporar cabeceras CSP estrictas en las plantillas HTML para bloquear la carga de scripts maliciosos inline o de orígenes desconocidos.
3. **Anti-Clickjacking**: Configurar la cabecera `X-Frame-Options: DENY` (o `SAMEORIGIN`) para impedir que la interfaz de Nebripop sea renderizada dentro de iframes maliciosos de terceros.
4. **HSTS Obligatorio**: Validar que se inyecta la cabecera `Strict-Transport-Security` en producción para forzar el uso de conexiones seguras HTTPS.

### Ejemplo de Configuración de Cabeceras en Axum

```rust
use axum::{
    middleware::{self, Next},
    response::Response,
    http::{Request, header, HeaderValue},
};

pub async fn apply_security_headers<B>(request: Request<B>, next: Next<B>) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // 1. Prevenir Clickjacking
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    
    // 2. Prevenir XSS e Inyecciones de script
    headers.insert(
        header::CONTENT_SECURITY_POLICY, 
        HeaderValue::from_static("default-src 'self'; script-src 'self' https://cdn.tailwindcss.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src https://fonts.gstatic.com;")
    );

    // 3. Forzar HTTPS en producción (HSTS)
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains; preload")
    );

    // 4. Prevenir Content Sniffing
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));

    response
}
```

---

## 7. Checklist de Auditoría por Módulo (Matriz Operativa)

Para realizar una auditoría completa del backend y frontend de Nebripop, se debe verificar de forma sistemática el siguiente checklist por módulo:

| Módulo | Elemento Crítico | Control de Seguridad Obligatorio |
|--------|------------------|----------------------------------|
| **`auth`** | Hashing de Contraseñas | ¿Usa Argon2id con parámetros OWASP? |
| | Token JWT | ¿Algoritmo HS256/RS256 sin 'None' y validación de expiración? |
| | Acceso a Rutas | ¿Middleware de sesión valida rol y autenticidad? |
| | Control de Fuerza Bruta | ¿Rate limiting activo en `/auth/login` y `/auth/register`? |
| **`listings`** | Subida de Imágenes | ¿Tamaño limitado a 10MB y Magic Bytes validados (no extensiones)? |
| | Edición de Anuncios | ¿Comprobación en backend de que `listing.seller_id == AuthUser.id`? |
| **`payments`** | Webhook Stripe | ¿Firma `Stripe-Signature` validada con clave secreta del webhook? |
| | Datos de Pago | ¿Absolutamente cero tarjetas de crédito almacenadas en BD? |
| | Duplicaciones | ¿Uso de Claves de Idempotencia en cobros? |
| **`chat`** | Websocket Handshake | ¿Token validado por query string al levantar socket? |
| | Acceso a Mensajes | ¿Validación en BD de pertenencia de usuario a la conversación? |
| | Sanitización | ¿Uso de `ammonia` en mensajes antes de persistir (mitigación XSS)? |
| **`search`** | Visibilidad de Datos | ¿Los resultados no exponen emails o datos internos sensibles de los usuarios? |

---

## 8. Metodología de Reporte de Vulnerabilidades

Cuando realices tareas de auditoría en el código y encuentres una debilidad o fallo de seguridad, debes suspender la escritura de código y estructurar un reporte formal de vulnerabilidades.

### Estructura de Reporte de Vulnerabilidad
El reporte debe redactarse como un artefacto markdown en el directorio de la conversación bajo el siguiente formato:

```markdown
# [SEC-AUDIT-01] - TÍTULO DE LA VULNERABILIDAD ENCONTRADA

* **Severidad**: [Crítica | Alta | Media | Baja]
* **Impacto**: Explicación de qué puede conseguir un atacante explotando este fallo.
* **Componente Afectado**: Ruta del fichero y líneas exactas.

## Descripción del Fallo
Explicación detallada de la debilidad con el código vulnerable actual.

## Prueba de Concepto (PoC)
Pasos o payload para reproducir el exploit.

## Mitigación Recomendada
Instrucciones detalladas de refactorización y parcheo de seguridad.
```

---

## 9. Las 15 Reglas Críticas de Seguridad para Nebripop

1. **Parámetros Estrictos Argon2id**: Las contraseñas se hashearán obligatoriamente con Argon2id configurando `m_cost = 19456` KB, `t_cost = 2` y `p_cost = 1`.
2. **Cero None en JWT**: Bloquea y rechaza de forma explícita el algoritmo `None` en la configuración de decodificación de `jsonwebtoken`. Exige exclusivamente `HS256`.
3. **Rate Limiting en Puertas de Acceso**: Todo handler de login o registro de usuarios debe contar con protección de rate-limiting activo en Axum para evitar fuerza bruta.
4. **Validación de Identidad en Servidor**: El backend nunca debe confiar en roles o identidades indicadas por el cliente. Toda autenticación debe validarse y verificarse en el backend a través del extractor `AuthUser`.
5. **Logs Libres de Secretos**: Está terminantemente prohibido loguear contraseñas, tokens JWT, cookies de sesión o PAN de tarjetas bancarias. Sanitiza las variables antes de invocar `tracing`.
6. **Entornos Aislados de Secretos**: Ninguna clave secreta o cadena de conexión (`DATABASE_URL`, `JWT_SECRET`, `STRIPE_SECRET_KEY`) debe estar hardcodeada. Inyéctalas exclusivamente mediante variables de entorno del sistema.
7. **Exclusión de .env**: El archivo `.env` local debe estar registrado obligatoriamente en el fichero `.gitignore` raíz para prevenir subidas accidentales a repositorios públicos.
8. **Consultas SQL Parametrizadas**: Queda estrictamente prohibida la concatenación de variables en consultas SQL (ej: `format!`). Usa placeholders nativos (`$1`, `$2`) en macros compiladas de SQLx.
9. **Inspección de Firma de Archivo (Magic Bytes)**: La subida de imágenes de productos debe validar los bytes de cabecera reales del archivo (PNG/JPEG) usando crates de firma mágica. Rechaza validaciones basadas solo en la extensión del fichero.
10. **Límite de Tamaño de Carga**: Configura un límite de tamaño físico estricto de máximo 10MB para cualquier carga de imagen de anuncio a través del servidor.
11. **Validación Obligatoria de Webhooks Stripe**: Todo endpoint que escuche eventos de Stripe debe verificar la firma del payload mediante la cabecera `Stripe-Signature` y la clave de webhook secreta.
12. **Idempotencia Transaccional**: Incorpora claves de idempotencia en las intenciones de cobro a Stripe para evitar transacciones duplicadas por caídas accidentales de red.
13. **Sanitización Activa de Chats**: Todos los mensajes de texto enviados mediante el chat WebSocket o polling tradicional deben ser sanitizados con `ammonia` antes de ser almacenados en base de datos.
14. **Autorización estricta de Canales de Chat**: Antes de emitir o retornar mensajes de una conversación, valida en PostgreSQL que el `UserId` solicitante sea participante (`buyer_id` o `seller_id`) de dicho chat.
15. **Cabeceras de Seguridad Globales**: Configura un middleware global en Axum que inyecte las cabeceras `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Content-Security-Policy` estricta y `Strict-Transport-Security` en producción.
