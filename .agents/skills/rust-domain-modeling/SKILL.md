---
name: rust-domain-modeling
description: Directrices de modelado de dominio rico, diseño táctico de Domain-Driven Design (DDD) y modelado de tipos en Rust para Nebripop. Utiliza esta skill siempre que vayas a escribir, modificar o auditar entidades de dominio, DTOs de API, Value Objects, Newtypes o agregados de negocio.
---

# Rich Domain Modeling in Rust — Nebripop

Esta skill define las directrices y estándares para estructurar el modelo de dominio de **Nebripop** utilizando el sistema de tipos fuerte de Rust. Aplicaremos técnicas avanzadas de **Domain-Driven Design (DDD)** táctico para garantizar que la lógica de negocio esté totalmente libre de estados inválidos, autovalidada en tiempo de compilación ("Parse, don't validate"), y estructurada mediante tipos de valor (Value Objects) desacoplados.

---

## 1. El Patrón Newtype para Identificadores de Entidad

Para prevenir la *Obsesión por Primitivos* y evitar mezclar accidentalmente identificadores distintos, cada entidad del dominio debe definir su propio struct de ID único envolviendo un `uuid::Uuid`. Está estrictamente prohibido usar `Uuid` o `i64` crudos en las firmas de dominio.

### Implementación Estándar de IDs

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ListingId(pub uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub uuid::Uuid);
```

---

## 2. Hacer Imposibles los Estados Inválidos (Domain Enums)

Evitaremos a toda costa representar estados operativos complejos o categorías mediante cadenas de texto libres (`String`). Utilizaremos enums estrictos para forzar al compilador a garantizar la exhaustividad de los flujos de negocio.

### Enums Clave del PRD

```rust
// Estado físico de los anuncios (PRD 6.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalCondition {
    New,
    LikeNew,
    Used,
}

// Estado del ciclo de vida del anuncio (PRD 6.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingStatus {
    Active,
    Sold,
    Reserved,
    Deleted,
}

// Categorías oficiales de Nebripop (PRD v1.0)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Fashion,
    Electronics,
    Home,
    Sports,
    Other,
}
```

---

## 3. Parse, Don't Validate (Construcción con Garantía de Validez)

Seguiremos la filosofía de diseño *"Parse, don't validate"*: **no validamos datos de forma dispersa a lo largo del código de negocio; en su lugar, transformamos (parseamos) los inputs crudos en Tipos de Valor fuertemente validados en su momento de creación.** Si un struct existe en memoria, es por definición estructuralmente válido.

### Ejemplo: El Value Object `Email`

```rust
use validator::validate_email;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    pub fn new(raw_email: String) -> Result<Self, &'static str> {
        let trimmed = raw_email.trim();
        if trimmed.is_empty() {
            return Err("El correo electrónico no puede estar vacío");
        }
        if !validate_email(trimmed) {
            return Err("El formato del correo electrónico es inválido");
        }
        Ok(Self(trimmed.to_lowercase()))
    }

    // Permitir extraer el valor seguro interno
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

---

## 4. Value Objects Complejos del PRD

### A. El Tipo de Valor `Price` (Decimal Sin Precisión Flotante)
Para prevenir pérdidas de centavos por precisión binaria (prohibido usar `f32`/`f64` en dinero), usaremos `rust_decimal::Decimal` validado para garantizar importes mayores a cero.

```rust
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Price(Decimal);

impl Price {
    pub fn new(amount: Decimal) -> Result<Self, &'static str> {
        if amount <= Decimal::ZERO {
            return Err("El precio del producto debe ser estrictamente superior a 0");
        }
        Ok(Self(amount))
    }

    pub fn value(&self) -> Decimal {
        self.0
    }
}
```

### B. El Tipo de Valor `Description` (Límite de Texto)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Description(String);

impl Description {
    pub fn new(text: String) -> Result<Self, &'static str> {
        let trimmed = text.trim();
        if trimmed.len() > 5000 {
            return Err("La descripción no puede superar los 5000 caracteres");
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

---

## 5. Estructuras Segregadas por Operación (DTOs de API vs Entidades)

Una entidad de dominio rico representa el estado actual del negocio. Está prohibido reutilizar la entidad de negocio para mapear los payloads que envía o recibe el cliente web. Cada operación requiere sus propios DTOs específicos.

### Ejemplo: Módulo de Anuncios (`listings`)

```rust
// 1. Entidad de Dominio Puro
pub struct Listing {
    pub id: ListingId,
    pub seller_id: UserId,
    pub title: String,
    pub description: Description,
    pub price: Price,
    pub condition: PhysicalCondition,
    pub status: ListingStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// 2. DTO de Entrada para Creación (Deserializable desde el JSON de la API)
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")] // Estándar de la API web
pub struct CreateListingDto {
    pub title: String,
    pub description: String,
    pub price_cents: i64, // Dinero ingresado de forma segura en céntimos
    pub condition: PhysicalCondition,
}

// 3. DTO de Entrada para Modificación (Todos los campos son opcionales)
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateListingDto {
    pub title: Option<String>,
    pub description: Option<String>,
    pub price_cents: Option<i64>,
    pub condition: Option<PhysicalCondition>,
    pub status: Option<ListingStatus>,
}

// 4. DTO de Respuesta de la API (Serializable a JSON para el cliente)
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingResponseDto {
    pub id: uuid::Uuid,
    pub seller_id: uuid::Uuid,
    pub title: String,
    pub description: String,
    pub price: String, // Formateado amigable (ej: "120.00 €")
    pub condition: PhysicalCondition,
    pub status: ListingStatus,
}
```

---

## 6. Serde y Serialización en la API (`camelCase`)

El estándar visual de Nebripop en el frontend web (JavaScript) requiere que todos los objetos JSON utilicen la convención de nomenclatura **`camelCase`**. Rust requiere **`snake_case`**. 

> [!IMPORTANT]
> **Todos los DTOs de entrada y salida expuestos en los controladores de Axum deben llevar obligatoriamente la anotación `#[serde(rename_all = "camelCase")]`.**

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // Convierte "first_image_url" a "firstImageUrl" automáticamente
pub struct UserProfileDto {
    pub user_id: uuid::Uuid,
    pub display_name: String,
    pub first_image_url: Option<String>,
    pub total_reviews: i32,
}
```

---

## 7. Modelado de Relaciones sin Acoplamiento de Módulos

Para evitar dependencias cruzadas entre los submódulos de la arquitectura hexagonal por crates de Nebripop (ej: que la entidad `Listing` requiera compilar el crate `users` completo porque contiene un struct `User` en su interior), **modelaremos las relaciones exclusivamente mediante IDs fuertemente tipados (Newtypes)**.

### Modelado Desacoplado de Relaciones

```rust
// crates/listings/src/domain.rs
use crate::models::ListingId;
// Importamos únicamente el Newtype UserId de users de forma desacoplada
use users::models::UserId; 

pub struct Listing {
    pub id: ListingId,
    pub seller_id: UserId, // Relación desacoplada por ID. Cero acoplamiento de layouts!
    pub title: String,
    // ...
}
```

---

## 8. Patrones Correctos vs. Incorrectos

### A. Obsesión por Primitivos en Entidades

❌ **Incorrecto (Uso de tipos primitivos crudos expuestos a errores de asignación cruzada)**
```rust
// ¿Cuál UUID es cuál? El compilador de Rust no impedirá que asocies seller_id a buyer_id por error
pub struct Conversation {
    pub id: uuid::Uuid,
    pub listing_id: uuid::Uuid,
    pub buyer_id: uuid::Uuid,
    pub seller_id: uuid::Uuid,
}
```

✅ **Correcto (Identificadores específicos mediante el patrón Newtype)**
```rust
pub struct Conversation {
    pub id: ConversationId,
    pub listing_id: ListingId,
    pub buyer_id: UserId,
    pub seller_id: UserId,
}
```

---

### B. Validación Reactiva vs Parse, don't validate

❌ **Incorrecto (Crear structs con campos planos y arrastrar validaciones manuales dispersas a lo largo de los casos de uso)**
```rust
pub struct ListingRaw {
    pub price: f64, // ¡Flotante peligroso!
    pub description: String,
}

// Caso de uso saturado de comprobaciones manuales redundantes
pub async fn create_listing_bad(listing: ListingRaw) -> Result<(), &'static str> {
    if listing.price <= 0.0 {
        return Err("Precio inválido");
    }
    if listing.description.len() > 5000 {
        return Err("Descripción muy larga");
    }
    // ...
    Ok(())
}
```

✅ **Correcto (Value Objects auto-validados en el momento de instanciación)**
```rust
pub struct ListingRich {
    pub price: Price,              // Ya validado y encapsulado con Decimal
    pub description: Description,  // Ya validado bajo longitud máxima
}
```

---

## 9. Las 12 Reglas Críticas de Modelado de Dominio para Nebripop

1. **Patrón Newtype Obligatorio para IDs**: Cada entidad de Nebripop debe definir su propio struct de ID único envolviendo un `uuid::Uuid` (ej. `UserId`, `ListingId`). Queda prohibido el uso de `Uuid` directo en el dominio.
2. **Cero Números Flotantes en Precios**: Los valores monetarios y precios del catálogo del PRD deben representarse exclusivamente con el tipo `Price` envolviendo `rust_decimal::Decimal`. Está prohibido usar `f32` o `f64`.
3. **Parse, Don't Validate**: Implementa constructores (`new()`) en los Value Objects que validen la integridad estructural y retornen un `Result<Self, Error>` en caliente. Garantiza que no puedan existir objetos de dominio inválidos en memoria.
4. **camelCase en la API**: Todos los DTOs de entrada o respuesta JSON definidos para los controladores de Axum deben llevar obligatoriamente la anotación `#[serde(rename_all = "camelCase")]`.
5. **Segregación de Structs por Operación**: Está prohibido reutilizar un único struct de dominio para todas las fases. Crea DTOs separados para la creación (`CreateListingDto`), actualización (`UpdateListingDto`) y respuesta (`ListingResponseDto`).
6. **Enums para Estados Operativos**: Reemplaza cualquier representación textual de estado por enums estructurados (`ListingStatus`, `PhysicalCondition`, `Category`) para forzar la comprobación exhaustiva del compilador.
7. **Desacoplamiento Relacional**: Representa las relaciones entre entidades de distintos módulos (ej: el propietario de un anuncio o el emisor de un mensaje) únicamente a través de los Newtypes de ID correspondientes (`UserId`).
8. **Restricción estricta en Description**: El Value Object `Description` debe validar y rechazar en su constructor cualquier texto que supere el límite físico de 5000 caracteres establecido.
9. **Precisión Geolocalizada**: El Value Object `GeoLocation` debe validar estrictamente que la latitud se sitúe entre `-90.0` y `90.0` y la longitud entre `-180.0` y `180.0`.
10. **Inmutabilidad por Defecto**: Mantén los campos de las entidades de dominio y Value Objects de forma privada por defecto, proporcionando métodos de lectura (`getter`) explícitos para proteger el estado interno.
11. **Tipado de Errores de Construcción**: Los errores devueltos por los constructores de los Value Objects deben acoplarse con enums tipados del dominio en lugar de retornar cadenas de texto planas.
12. **Derivación Homogénea de Rasgos**: Todos los Newtypes de ID y Value Objects deben derivar de forma consistente los traits estándar de Rust: `Debug`, `Clone`, `PartialEq`, `Eq` y `Hash`.
