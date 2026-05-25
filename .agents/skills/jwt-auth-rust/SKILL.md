---
name: jwt-auth-rust
description: Directrices de arquitectura, mejores prácticas y patrones de codificación para implementar la autenticación JWT y el hashing de contraseñas con Argon2id en el backend de Rust + Axum de Nebripop. Utiliza esta skill siempre que vayas a escribir, modificar o auditar endpoints de autenticación, extractores de tokens, hashing de contraseñas o políticas de permisos por rol.
---

# JWT Authentication & Password Hashing — Nebripop

Esta skill define las directrices y estándares de seguridad para implementar la autenticación mediante JSON Web Tokens (JWT) y el hashing de contraseñas con **Argon2id** en el backend de **Nebripop**. El objetivo es garantizar un sistema de identidad robusto, inmune a ataques de fuerza bruta y de filtración de datos, que cumpla estrictamente con las políticas de permisos por rol descritas en el PRD.

---

## 1. Hashing de Contraseñas con Argon2id (Parámetros OWASP)

Las contraseñas de los usuarios en la tabla `users` nunca deben almacenarse en texto plano ni hashearse con algoritmos obsoletos (MD5, SHA256, bcrypt). Se utilizará **Argon2id** (variante segura recomendada para almacenar contraseñas) parametrizado bajo las recomendaciones de OWASP para prevenir ataques de hardware dedicado (ASIC/GPU).

### Parámetros OWASP Recomendados
* **m_cost (Memoria)**: `19456` KB (19 MB).
* **t_cost (Iteraciones)**: `2` rondas.
* **p_cost (Paralelismo)**: `1` hilo.

### Implementación Segura en Rust (`argon2`)

```rust
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params, Version,
};

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    
    // Configurar parámetros recomendados por OWASP
    let params = Params::new(19456, 2, 1, None)?;
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        Version::V13,
        params
    );

    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(password_hash.to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(_) => return false,
    };

    // Argon2 deduce los parámetros automáticamente desde el hash almacenado
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}
```

---

## 2. Estructura y Generación de JWT (`jsonwebtoken`)

El backend de Nebripop firma y verifica de forma simétrica tokens JWT utilizando el algoritmo **HS256** junto a una variable de entorno secreta `JWT_SECRET`.

### Qué debe y qué NO debe ir en el Payload (Claims)

> [!IMPORTANT]
> **El JWT es visible en el cliente (Base64). Nunca debes incluir contraseñas, hashes, correos electrónicos completos, datos bancarios o información personal confidencial en el payload.**

* **Sí debe ir (Identificadores y Permisos mínimos)**:
  * `sub` (Subject): El ID único del usuario (`uuid::Uuid`).
  * `role` (Role): Rol del usuario para autorización rápida (`"user"` o `"admin"`).
  * `exp` (Expiration): Timestamp de expiración obligatorio (máximo 24h para login, 15 min para tokens temporales).
  * `iat` (Issued At): Timestamp de creación.
* **No debe ir (Seguridad y Tamaño)**:
  * Contraseñas o hashes (`password_hash`).
  * Datos personales altamente mutables (nombre, bio, avatar, ciudad).
  * Payloads masivos que saturen el ancho de banda del canal HTTP.

### Definición y Creación de Claims en Rust

```rust
use serde::{Serialize, Deserialize};
use jsonwebtoken::{Header, EncodingKey, DecodingKey, Validation};
use chrono::{Utc, Duration};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: uuid::Uuid,     // ID del usuario
    pub role: String,        // "user" o "admin"
    pub exp: i64,            // Fecha de expiración (Timestamp UTC)
    pub iat: i64,            // Fecha de creación
}

pub fn generate_jwt(user_id: uuid::Uuid, role: &str, jwt_secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let expiration = now + Duration::hours(24); // Expiración recomendada: 24 horas

    let claims = Claims {
        sub: user_id,
        role: role.to_string(),
        exp: expiration.timestamp(),
        iat: now.timestamp(),
    };

    let token = jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )?;

    Ok(token)
}
```

---

## 3. Extractor Personalizado de Autenticación (`AuthUser`)

Para proteger endpoints en Axum 0.7, definimos el extractor personalizado `AuthUser` que automáticamente extrae e inspecciona el token JWT en las peticiones entrantes. Si el token está ausente, expirado o mal firmado, aborta la petición retornando un error unificado `401 Unauthorized` legible para el frontend.

```rust
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};
use jsonwebtoken::{decoding_key::DecodingKey, Validation};

pub struct AuthUser {
    pub id: uuid::Uuid,
    pub role: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
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
            .ok_or(AppError::Unauthorized("Cabecera Authorization no encontrada".to_string()))?;

        // 2. Comprobar prefijo "Bearer "
        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized("El token debe usar el formato Bearer".to_string()));
        }

        let token = &auth_header[7..];
        let app_state = AppState::from_ref(state);

        // 3. Decodificar y Validar el JWT
        let token_data = jsonwebtoken::decode::<Claims>(
            token,
            &DecodingKey::from_secret(app_state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|err| {
            // Manejo claro de tokens expirados
            if let jsonwebtoken::errors::ErrorKind::ExpiredSignature = err.kind() {
                AppError::Unauthorized("El token de sesión ha expirado".to_string())
            } else {
                AppError::Unauthorized("Token de sesión inválido".to_string())
            }
        })?;

        Ok(AuthUser {
            id: token_data.claims.sub,
            role: token_data.claims.role,
        })
    }
}
```

---

## 4. Tabla de Permisos por Rol (Sección 3 del PRD)

El extractor `AuthUser` permite controlar el acceso a nivel de handler. Todo endpoint protegido debe validar la propiedad del recurso (ej: el comprador solo edita su perfil, el vendedor edita su anuncio propio) o validar si el rol es `admin`.

| Ruta / Endpoint | Método | Requisito de Sesión | Validación de Propiedad / Rol |
|-----------------|:------:|:--------------------:|-------------------------------|
| `/auth/register` | `POST` | Anónimo | Ninguno |
| `/auth/login` | `POST` | Anónimo | Ninguno |
| `/listings` | `POST` | Autenticado | Asignar `seller_id` = `AuthUser.id` |
| `/listings/:id` | `PUT` | Autenticado | Solo propietario (`seller_id` == `AuthUser.id`) o Admin |
| `/listings/:id` | `DELETE` | Autenticado | Solo propietario (`seller_id` == `AuthUser.id`) o Admin |
| `/chat` | `POST` | Autenticado | Ninguno |
| `/chat` | `GET` | Autenticado | Cargar solo conversaciones donde `buyer_id` o `seller_id` == `AuthUser.id` |
| `/payments` | `POST` | Autenticado | El comprador no puede ser el propietario del anuncio |
| `/users/:id` | `PUT` | Autenticado | Solo el propio usuario (`id` == `AuthUser.id`) |
| `/users/:id` | `DELETE` | Autenticado | Requiere rol `AuthUser.role == "admin"` |

---

## 5. Diseño de Endpoints de Autenticación

### A. Registro (`POST /auth/register`)
```rust
#[derive(Deserialize, Validate)]
pub struct RegisterDto {
    #[validate(email(message = "Email no válido"))]
    pub email: String,
    #[validate(length(min = 8, message = "La contraseña debe tener al menos 8 caracteres"))]
    pub password: String,
    #[validate(length(min = 2, message = "El nombre de visualización debe tener al menos 2 caracteres"))]
    pub display_name: String,
}

pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    payload.validate().map_err(AppError::ValidationError)?;

    // 1. Hashear contraseña con Argon2id de forma segura
    let password_hash = hash_password(&payload.password)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 2. Persistir usuario (delega a la capa de persistencia)
    let user = users::usecases::create_user(
        &payload.email,
        &password_hash,
        &payload.display_name,
        &state.db
    ).await?;

    // 3. Generar JWT para el inicio inmediato
    let token = generate_jwt(user.id, &user.role, &state.jwt_secret)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(AuthResponse { token, user: UserDto::from_domain(user) })))
}
```

### B. Inicio de Sesión (`POST /auth/login`)
```rust
#[derive(Deserialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginDto>,
) -> Result<Json<AuthResponse>, AppError> {
    // 1. Obtener usuario de la base de datos por email
    let user = users::usecases::find_user_by_email(&payload.email, &state.db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Credenciales incorrectas".to_string()))?;

    // 2. Verificar contraseña de forma segura frente al hash Argon2id
    if !verify_password(&payload.password, &user.password_hash) {
        return Err(AppError::Unauthorized("Credenciales incorrectas".to_string()));
    }

    // 3. Generar token
    let token = generate_jwt(user.id, &user.role, &state.jwt_secret)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(AuthResponse { token, user: UserDto::from_domain(user) }))
}
```

### C. Refresco de Token (`POST /auth/refresh`)
Para extender la sesión sin forzar al usuario a loguearse constantemente. Exige enviar un token válido y genera uno nuevo renovado por 24 horas.

```rust
pub async fn refresh_handler(
    State(state): State<AppState>,
    auth_user: AuthUser, // Extractor exige que el token actual siga siendo válido
) -> Result<Json<RefreshResponse>, AppError> {
    // Generar un nuevo token fresco
    let new_token = generate_jwt(auth_user.id, &auth_user.role, &state.jwt_secret)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(RefreshResponse { token: new_token }))
}
```

---

## 6. Patrones Correctos vs. Incorrectos

### A. Hashing de Contraseñas

❌ **Incorrecto (MD5/SHA sin salting o uso de hashes rápidos. Vulnerable a ataques de diccionario y arcoíris)**
```rust
// TOTALMENTE PROHIBIDO
use sha2::{Sha256, Digest};

pub fn insecure_hash(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

✅ **Correcto (Argon2id con salting aleatorio criptográfico y parámetros OWASP seguros)**
```rust
pub fn secure_hash(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(19456, 2, 1, None)?; // OWASP
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V13, params);
    
    Ok(argon2.hash_password(password.as_bytes(), &salt)?.to_string())
}
```

---

### B. Payload de Datos del JWT (Exceso de Información)

❌ **Incorrecto (Almacenar datos confidenciales o de tamaño variable en el payload del JWT. Afecta la seguridad y el rendimiento HTTP)**
```rust
#[derive(Serialize)]
pub struct InsecureClaims {
    pub sub: uuid::Uuid,
    pub password_hash: String, // ¡ERROR CRÍTICO: Hash expuesto públicamente!
    pub email: String,         // Datos personales
    pub bio: String,           // Campo de texto variable enorme
}
```

✅ **Correcto (Claims compactos, solo con identificadores lógicos e integridad temporal)**
```rust
#[derive(Serialize)]
pub struct SecureClaims {
    pub sub: uuid::Uuid,  // Solo ID
    pub role: String,     // Solo Rol
    pub exp: i64,         // Expiración obligatoria
    pub iat: i64,         // Creación
}
```

---

## 7. Las 12 Reglas Críticas de Autenticación para Nebripop

1. **Uso Exclusivo de Argon2id**: Las contraseñas de los usuarios deben procesarse única y exclusivamente mediante la variante `Argon2id`. Está prohibido el uso de SHA, MD5, bcrypt o pbkdf2.
2. **Cumplimiento de Parámetros OWASP**: La inicialización de Argon2id debe configurarse con los parámetros oficiales de seguridad de memoria y paralelismo (`m_cost = 19456`, `t_cost = 2`, `p_cost = 1`).
3. **Firma Criptográfica HS256**: Los JSON Web Tokens deben firmarse simétricamente mediante el algoritmo estándar `HS256` combinado con una clave segura `JWT_SECRET`.
4. **Cero Datos Sensibles en JWT**: Está prohibido incluir contraseñas, hashes, correos electrónicos o cualquier otro dato personal sensible dentro del payload de los claims del JWT.
5. **Caducidad Obligatoria**: Todo token de autenticación generado para inicio de sesión en Nebripop debe incluir obligatoriamente la propiedad `exp` (expiration) configurada a un máximo de 24 horas.
6. **Manejo Específico de Token Expirado**: Si la firma del token decodificado ha expirado (`ExpiredSignature`), el extractor debe capturarlo y retornar un error `401 Unauthorized` con el mensaje explícito: `"El token de sesión ha expirado"`.
7. **Extractor Centralizado AuthUser**: Protege los endpoints del backend que requieran sesión activa utilizando el extractor `AuthUser` en lugar de recuperar las cabeceras HTTP de forma manual en los controladores.
8. **Validación de Propiedad de Recurso**: Para peticiones de mutación (`PUT`, `DELETE` en listings o usuarios), los handlers deben comprobar obligatoriamente que el identificador del registro a modificar coincide con `AuthUser.id` (a menos que el rol sea `"admin"`).
9. **Cero Mocks de Criptografía**: Durante los tests de integración, no mockees los algoritmos de hashing ni de decodificación. Utiliza los flujos de criptografía reales para asegurar la correspondencia con producción.
10. **Sanitización de Errores de Login**: En el endpoint de inicio de sesión (`/auth/login`), si el email no existe o la contraseña es inválida, retorna un mensaje genérico unificado: `"Credenciales incorrectas"`. Esto evita ataques de enumeración de cuentas.
11. **Configuración Segura de JWT_SECRET**: En entornos de producción (`APP_ENV=production`), el servidor debe abortar inmediatamente si `JWT_SECRET` no está establecido o si coincide con la clave por defecto de desarrollo.
12. **Validación Estricta de Registro**: Todo DTO para el registro de nuevos usuarios (`RegisterDto`) debe validar obligatoriamente el formato correcto del campo email y requerir una contraseña con un mínimo de 8 caracteres mediante la macro `#[derive(Validate)]`.
