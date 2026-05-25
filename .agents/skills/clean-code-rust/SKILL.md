---
name: clean-code-rust
description: Directrices de Clean Code, legibilidad y estándares de diseño de código para el backend de Rust en Nebripop. Utiliza esta skill siempre que vayas a escribir, refactorizar o revisar código de handlers, usecases, modelos de dominio o adaptadores de base de datos.
---

# Clean Code & Code Quality in Rust — Nebripop

Esta skill define las directrices y estándares de legibilidad, mantenibilidad y diseño de código limpio para el desarrollo del backend de **Nebripop** en Rust. Su objetivo es garantizar un código legible por cualquier agente o desarrollador, estructurado de forma consistente, libre de pánicos y con una arquitectura de tipos de dominio fuerte.

---

## 1. Convenciones de Nomenclatura (Rust API Guidelines)

El código de Nebripop debe adherirse estrictamente a las convenciones de nomenclatura oficiales de la comunidad de Rust (*Rust API Guidelines*).

### Resumen de Estilos de Nomenclatura
* **`UpperCamelCase` (PascalCase)**: Structs, Enums, Traits y Union types (ej. `ListingId`, `CreateUserCommand`).
* **`snake_case`**: Variables, funciones, nombres de ficheros, módulos, crates y campos de structs (ej. `location_lat`, `find_listing_by_id`).
* **`SCREAMING_SNAKE_CASE`**: Constantes y variables globales estáticas (ej. `MAX_LISTING_IMAGES`).

### Prohibición de Abreviaciones
Para evitar ambigüedades en la lectura del código, se prohíbe el uso de abreviaciones perezosas.

* ❌ **Incorrecto**: `usr`, `lst`, `txn`, `msg`, `img`, `auth`, `id_usr`
* ✅ **Correcto**: `user`, `listing`, `transaction`, `message`, `image`, `authentication`, `user_id`

---

## 2. Patrón Newtype para Tipos de Dominio

El patrón *Newtype* (envolver un tipo primitivo en una tupla struct de un solo campo) es obligatorio para los identificadores y tipos numéricos clave de Nebripop. Esto previene la *Obsesión por Primitivos* y evita errores lógicos en tiempo de compilación (como pasar por error el UUID de un anuncio en el parámetro correspondiente a un ID de usuario).

### Definición de Tipos en el Dominio

```rust
use serde::{Serialize, Deserialize};

// 1. Identificadores de Entidad Fuertemente Tipados
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ListingId(pub uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(pub uuid::Uuid);

// 2. Tipos de Valor Complejos
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Price(pub rust_decimal::Decimal);

impl Price {
    pub fn format_eur(&self) -> String {
        format!("{:.2} €", self.0)
    }
}
```

### Uso en las Firmas de Funciones de Dominio
```rust
// El compilador de Rust no te dejará compilar si mezclas UserId y ListingId
pub async fn register_favorite(
    user_id: UserId, 
    listing_id: ListingId, 
    db: &sqlx::PgPool
) -> Result<(), DomainError> { ... }
```

---

## 3. Tamaño y Responsabilidad Única (SRP) de Funciones

Para garantizar la mantenibilidad de Nebripop, las funciones deben ser pequeñas y tener una **única responsabilidad**. 

### La Regla de las 20 Líneas
> [!IMPORTANT]
> **Ninguna función de handler (Axum) ni caso de uso (Usecase) debe exceder las 20 líneas de código útil.** 

Si una función supera este límite, significa que está acumulando múltiples responsabilidades de bajo nivel y debe ser refactorizada extrayendo fragmentos a funciones privadas auxiliares.

### Cuándo Extraer Lógica a Funciones Auxiliares Privadas
* **Cálculos matemáticos o conversión**: Formatear textos, calcular distancias por Haversine o calcular comisiones.
* **Construcción de Payloads**: Crear estructuras internas o mapeadores (`from_domain` / `into_dto`).
* **Validación compleja**: Comprobaciones personalizadas más allá del struct DTO de entrada.

---

## 4. Regla de los Tres Niveles de Abstracción Máximos (SLA)

Una función debe mantener un **Único Nivel de Abstracción** (Single Level of Abstraction). No mezcles lógica de orquestación de alto nivel con manipulación de bajo nivel (como parsear JSON, ejecutar SQL raw o calcular comisiones) en el mismo ámbito.

### Los Tres Niveles Máximos Permitidos en una Función
1. **Nivel Alto (Orquestación)**: Coordina llamadas a otros servicios o casos de uso.
2. **Nivel Medio (Lógica del Dominio)**: Ejecuta validaciones de negocio básicas y llamadas a interfaces de persistencia (repositorios).
3. **Nivel Bajo (Persistencia e Infraestructura)**: Queries SQL directas, lectura de buffers asíncronos o hashing criptográfico.

---

## 5. Sin Comentarios Obvios (Código Autodocumentado)

No agregues comentarios obvios que solo describan lo que hace el código en su superficie. **El buen código debe leerse como prosa inglesa.**

* ❌ **Comentario Inútil**: `// Comprobar si el anuncio está activo`
* ✅ **Código Autodocumentado**: `if listing.is_active() { ... }`

Usa comentarios **únicamente** para explicar decisiones de diseño no obvias ("el porqué") o consideraciones complejas de rendimiento y limitaciones de APIs externas.

---

## 6. Organización Estructurada de los `use` Statements

Los imports al inicio de cada archivo Rust deben agruparse en bloques limpios separados por saltos de línea para mejorar la legibilidad y evitar dependencias cruzadas desordenadas.

### Jerarquía Oficial de Imports de Nebripop
```rust
// Grupo 1: Librería Estándar (std)
use std::sync::Arc;
use std::time::Duration;

// Grupo 2: Dependencias Externas (Crates de terceros)
use axum::extract::State;
use serde::Deserialize;
use uuid::Uuid;

// Grupo 3: Módulos o Crates del Workspace (Lógica local de Nebripop)
use crate::models::{Listing, ListingId};
use crate::repositories::ListingRepository;
```

---

## 7. Patrones Correctos vs. Incorrectos (Ejemplos Comparativos)

### A. Firma del Handler y Nombres de Variable

❌ **Incorrecto (Bloque de código enorme, nombres abreviados, variables confusas e inseguras sin Newtype)**
```rust
// Handler incomprensible de 35 líneas
pub async fn upd_lst(
    State(st): State<AppState>,
    claims: Claims,
    Path(lid): Path<Uuid>,
    Json(pay): Json<UpdateDto>,
) -> Result<StatusCode, AppError> {
    // Abreviaciones inútiles y acoplamiento directo de SQLx
    let u = sqlx::query!("SELECT seller_id, status FROM listings WHERE id = $1", lid)
        .fetch_one(&st.db)
        .await
        .map_err(AppError::DatabaseError)?;

    // Lógica en el handler mezclando validaciones y strings
    if u.seller_id != claims.sub {
        return Err(AppError::Forbidden("no".to_string()));
    }

    if u.status != "active" {
        return Err(AppError::BadRequest("closed".to_string()));
    }

    sqlx::query!("UPDATE listings SET title = $1 WHERE id = $2", pay.t, lid)
        .execute(&st.db)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(StatusCode::OK)
}
```

✅ **Correcto (Código autodocumentado, Newtypes, orquestado y menor a 20 líneas)**
```rust
pub async fn update_listing_handler(
    State(state): State<AppState>,
    Path(listing_id): Path<Uuid>,
    auth_user: AuthUser,
    Json(payload): Json<UpdateListingDto>,
) -> Result<StatusCode, AppError> {
    // 1. Tipar fuertemente las primitivas de entrada
    let listing_id = ListingId(listing_id);
    let user_id = UserId(auth_user.id);

    // 2. Construir el comando de dominio
    let command = payload.into_command(listing_id, user_id);

    // 3. Invocar al Usecase (Lógica desacoplada y limpia)
    listings::usecases::update_listing_usecase(command, &state.db).await?;

    Ok(StatusCode::OK)
}
```

---

### B. Evitar la Obsesión de Tipos Primitivos

❌ **Incorrecto (Variables sueltas expuestas a confusiones en tiempo de compilación)**
```rust
// Si por error pasamos buyer_id en el parámetro de seller_id, compila sin problemas pero causa un error grave de negocio!
pub async fn register_rating(
    transaction_id: Uuid,
    rater_id: Uuid,
    rated_id: Uuid,
    score: i16,
    db: &PgPool
) -> Result<(), AppError> { ... }
```

✅ **Correcto (Estructura con Newtypes y tipos de dominio fuertemente validados)**
```rust
pub struct RatingScore(pub i16); // Tipo de valor validado

impl RatingScore {
    pub fn new(score: i16) -> Result<Self, &'static str> {
        if score < 1 || score > 5 {
            Err("La puntuación debe estar entre 1 y 5")
        } else {
            Ok(Self(score))
        }
    }
}

pub async fn register_rating(
    transaction_id: TransactionId,
    rater_id: UserId,
    rated_id: UserId,
    score: RatingScore,
    db: &PgPool
) -> Result<(), AppError> { ... }
```

---

### C. Refactorización para Funciones Auxiliares Privadas

❌ **Incorrecto (Handler que mezcla cálculo de tarifas de Stripe e inserciones SQLx. Excede las 20 líneas de código de forma innecesaria)**
```rust
pub async fn process_payment(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<PaymentDto>,
) -> Result<StatusCode, AppError> {
    // Lógica matemática inyectada en el controlador
    let amount_decimal = rust_decimal::Decimal::from_f64(payload.amount).unwrap();
    let fee = amount_decimal * rust_decimal::dec!(0.05); // 5% fee
    let total_charge = amount_decimal + fee;

    // Conexión directa a base de datos y Stripe...
    let payment_intent = stripe::PaymentIntent::create(
        &state.stripe_client,
        stripe::CreatePaymentIntent {
            amount: total_charge.to_i64().unwrap() * 100, // Céntimos
            currency: stripe::Currency::EUR,
            // ...
        }
    ).await.map_err(|e| AppError::StripeError(e.to_string()))?;

    // Inserción en base de datos directa...
    Ok(StatusCode::OK)
}
```

✅ **Correcto (Lógica de cálculo extraída a helpers y persistencia delegada al Usecase)**
```rust
pub async fn process_payment_handler(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<PaymentDto>,
) -> Result<Json<PaymentResponse>, AppError> {
    let buyer_id = UserId(auth_user.id);
    
    // 1. Extraer el cálculo matemático a una función pura y testeable
    let charge = calculate_platform_fees(payload.amount);

    // 2. Delegar la persistencia y llamadas externas (Stripe) al caso de uso de dominio
    let payment = payments::usecases::charge_buyer_usecase(
        buyer_id, 
        payload.listing_id, 
        charge, 
        &state.db, 
        &state.stripe_client
    ).await?;

    Ok(Json(PaymentResponse::from_domain(payment)))
}

// Función auxiliar pura y pequeña
fn calculate_platform_fees(amount: rust_decimal::Decimal) -> PlatformCharge {
    let fee = amount * rust_decimal::dec!(0.05);
    PlatformCharge {
        base_amount: amount,
        platform_fee: fee,
        total: amount + fee,
    }
}
```

---

## 8. Las 12 Reglas Críticas de Clean Code para Nebripop

1. **Sin Abreviaciones en Nombres**: Escribe nombres de variables, funciones y estructuras completos en inglés. Usa `user` (no `usr`), `listing` (no `lst`), `transaction` (not `txn`) y `message` (not `msg`).
2. **Uso del Patrón Newtype**: Toda ID de recurso (`UserId`, `ListingId`, `TransactionId`) y tipo numérico crítico de negocio (`Price`) debe representarse con un struct newtype para evitar mezclar datos en tiempo de compilación.
3. **Límite de 20 Líneas**: Ninguna función de handler HTTP de Axum ni de caso de uso (Usecase) debe exceder las 20 líneas útiles de código.
4. **SRP (Responsabilidad Única)**: Cada función, struct o módulo debe tener un único motivo de cambio. Si tu handler valida datos, calcula comisiones e inserta en la BD, está violando el SRP.
5. **Autodocumentación Obligatoria**: Nombra funciones y variables con verbos y nombres descriptivos en inglés. El código debe poder leerse sin comentarios explicativos superfluos de lo que hace.
6. **No Comentar lo Obvio**: Elimina cualquier comentario que describa mecánicamente lo que hace una línea o bloque de código simple (ej. `// Insertar anuncio`). Usa comentarios solo para explicar el "por qué".
7. **Imports Ordenados por Bloque**: Agrupa los `use` en tres bloques separados por un salto de línea: 1. Librería Estándar (`std`), 2. Crates de terceros (`axum`, `serde`), 3. Módulos locales del workspace (`crate::`).
8. **Funciones Puras para Lógica Compleja**: Extrae cualquier cálculo, formateo de datos complejos o mapeos estructurales a funciones puras auxiliares privadas para facilitar el testeo unitario.
9. **Cero Tipos Anónimos (Tuple Obfuscation)**: Evita retornar tuplas anónimas complejas (ej. `Result<(String, Uuid, i64, bool), Error>`). Define structs con nombres explícitos y semánticos.
10. **Alineación con Rust API Guidelines**: Utiliza `snake_case` para funciones y variables, `UpperCamelCase` para structs/enums, y `SCREAMING_SNAKE_CASE` para constantes.
11. **Tipado Fuertemente Acoplado a Validaciones**: Si un tipo numérico (como la puntuación `RatingScore` de 1 a 5) tiene restricciones de rango, valídalo de inmediato en su constructor (`new()`) en lugar de arrastrar comprobaciones sueltas.
12. **Propagación del Contexto de Error**: Utiliza las facilidades del crate `thiserror` para modelar y propagar errores tipados del dominio en lugar de usar cadenas de texto genéricas (`Result<T, String>`).
