---
name: hexagonal-architecture-rust
description: Directrices de arquitectura, mejores prácticas y patrones de codificación para la separación por crates de Nebripop siguiendo Arquitectura Hexagonal (Puertos y Adaptadores) en Rust. Utiliza esta skill siempre que vayas a estructurar el workspace Cargo, definir un trait en el dominio (puerto), implementar un repositorio con SQLx (adaptador), configurar el AppState de Axum, definir dependencias entre crates o diseñar el flujo de datos desacoplado del backend.
---

# Skill de Arquitectura Hexagonal en Rust — Nebripop

Esta skill proporciona las directrices absolutas de arquitectura, estructuración física y diseño técnico para Nebripop. El backend del sistema se construye como un **Workspace Cargo multi-crate**, implementando de forma estricta el patrón **Arquitectura Hexagonal (Puertos y Adaptadores)**.

---

## Estructura Exacta del Workspace Cargo

Nebripop está dividido en **9 crates** independientes para garantizar un desacoplamiento total y permitir compilaciones paralelas ultra-rápidas:

```text
nebripop/ (Raíz del proyecto)
├── Cargo.toml (Workspace Config)
├── crates/
│   ├── users/        # Dominio, puertos y lógica de Usuarios y Auth
│   ├── listings/     # Dominio, puertos y lógica de Productos/Anuncios
│   ├── search/       # Crate para indexación e integración con MeiliSearch
│   ├── chat/         # Mensajería instantánea en tiempo real (WebSockets)
│   ├── payments/     # Orquestación de pagos y pasarela con Stripe
│   ├── ratings/      # Calificaciones y reseñas entre usuarios
│   ├── favorites/    # Gestión de favoritos y listas de deseos
│   ├── geo/          # Cálculos de geolocalización y rangos de distancia
│   └── api/          # Composición raíz, Routers y Handlers de Axum (HTTP)
```

---

## 📂 Organización Interna de Archivos en cada Crate de Dominio

A excepción del crate orquestador `api`, cada crate de dominio (por ejemplo, `crates/users`) debe estructurarse físicamente de la siguiente manera:

```text
crates/users/
├── Cargo.toml
└── src/
    ├── lib.rs        # Exporta únicamente los módulos públicos necesarios
    ├── domain/       # CAPA INTERNA: Modelos puros y lógica de negocio
    │   ├── mod.rs
    │   ├── user.rs   # Struct User pura
    │   └── errors.rs # Enum de errores del dominio (UserError)
    ├── ports/        # CAPA INTERNA: Puertos (Traits) que definen interfaces
    │   ├── mod.rs
    │   └── repository.rs # Trait UserRepository
    └── adapters/     # CAPA EXTERNA: Implementaciones concretas de infraestructura
        ├── mod.rs
        ├── postgres_repository.rs # Implementación con SQLx (SqlxUserRepository)
        └── memory_repository.rs   # Opcional: Para testing unitario veloz
```

---

## 🛠️ Diseño de Puertos (Ports) y Adaptadores (Adapters)

### 1. Definición del Puerto (Port) en el Dominio
Los puertos se definen en la carpeta `ports/` de su crate respectivo como **Traits asíncronos** de Rust. No conocen nada sobre bases de datos específicas (`sqlx`, `postgres`, etc.).

```rust
// crates/users/src/ports/repository.rs
use async_trait::async_trait;
use crate::domain::{user::User, errors::UserError};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: i32) -> Result<User, UserError>;
    async fn find_by_email(&self, email: &str) -> Result<User, UserError>;
    async fn create(&self, user: &User) -> Result<User, UserError>;
}
```

### 2. Implementación del Adaptador (Adapter)
El adaptador se define en la carpeta `adapters/` e implementa el trait del puerto utilizando infraestructura real.

```rust
// crates/users/src/adapters/postgres_repository.rs
use async_trait::async_trait;
use sqlx::PgPool;
use crate::domain::{user::User, errors::UserError};
use crate::ports::repository::UserRepository;

pub struct SqlxUserRepository {
    pool: PgPool,
}

impl SqlxUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for SqlxUserRepository {
    async fn find_by_id(&self, id: i32) -> Result<User, UserError> {
        let result = sqlx::query_as!(
            User,
            "SELECT id, name, email, password_hash, created_at FROM users WHERE id = $1",
            id
        )
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(user) => Ok(user),
            Err(sqlx::Error::RowNotFound) => Err(UserError::NotFound),
            Err(e) => Err(UserError::DatabaseError(e.to_string())),
        }
    }

    async fn find_by_email(&self, email: &str) -> Result<User, UserError> {
        // Implementación...
        todo!()
    }

    async fn create(&self, user: &User) -> Result<User, UserError> {
        // Implementación...
        todo!()
    }
}
```

---

## 🎛️ Orquestación e Inyección de Dependencias en el Crate `api`

El crate `api` actúa como la **Raíz de Composición (Composition Root)**. Es el único que depende de todos los adaptadores específicos. Compila las dependencias concretas, instancia las bases de datos e inyecta los adaptadores en el estado de la aplicación (`AppState`) de Axum usando punteros inteligentes `Arc<dyn Trait>` para asegurar el polimorfismo.

```rust
// crates/api/src/state.rs
use std::sync::Arc;
use users::ports::repository::UserRepository;
use listings::ports::repository::ListingRepository;

#[derive(Clone)]
pub struct AppState {
    // Inyección de dependencias abstractas:
    pub user_repo: Arc<dyn UserRepository>,
    pub listing_repo: Arc<dyn ListingRepository>,
}
```

### Configuración del Servidor en `crates/api/src/main.rs`:

```rust
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use axum::{routing::get, Router};
use api::state::AppState;
use users::adapters::postgres_repository::SqlxUserRepository;
use listings::adapters::postgres_repository::SqlxListingRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new().connect(&database_url).await?;

    // 1. Instanciar adaptadores de infraestructura concretos
    let user_adapter = SqlxUserRepository::new(pool.clone());
    let listing_adapter = SqlxListingRepository::new(pool.clone());

    // 2. Inyectar en el AppState envolviéndolos en Arc
    let shared_state = AppState {
        user_repo: Arc::new(user_adapter),
        listing_repo: Arc::new(listing_adapter),
    };

    // 3. Construir routers y pasar estado
    let app = Router::new()
        .route("/health", get(health_handler))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}
```

---

## 📐 Las 12 Reglas Estrictas de la Arquitectura Hexagonal

El agente debe hacer cumplir rigurosamente las siguientes 12 reglas arquitectónicas durante todo el desarrollo:

### 1. Pureza Absoluta del Dominio (Sin `axum` ni `sqlx`)
La capa `domain/` de cualquier crate no debe tener conocimiento de cómo se transportan los datos (HTTP, gRPC, WebSocket) ni de cómo se almacenan (PostgreSQL, Redis, MeiliSearch).
*   ❌ **Incorrecto**: Importar `axum::Json` o utilizar anotaciones `#[derive(sqlx::FromRow)]` directamente en las structs ubicadas en `crates/users/src/domain/user.rs`.
*   ✔️ **Correcto**: Mantener structs de Rust estándar e independientes. Definir structs de DTO de serialización en la capa HTTP (`api`) o mapear manualmente filas de la base de datos en los adaptadores.

### 2. Inversión de Control Estricta mediante Puertos
Toda interacción con servicios externos o persistencia debe realizarse a través de un Puerto (Trait). El dominio jamás debe invocar una base de datos o API directamente.
*   ❌ **Incorrecto**: Instanciar y llamar a `SqlxUserRepository` o hacer un query crudo a Postgres desde un servicio de dominio en `users`.
*   ✔️ **Correcto**: El caso de uso en el dominio recibe un parámetro genérico o de tipo `Arc<dyn UserRepository>` y opera exclusivamente sobre los métodos del trait.

### 3. Dirección Unidireccional del Flujo de Dependencia
Las dependencias deben fluir únicamente hacia adentro. Las capas externas (`adapters`, `api`) conocen el dominio y los puertos; el dominio jamás conoce la infraestructura o el transporte.
*   ❌ **Incorrecto**: El archivo `crates/users/src/domain/user.rs` importa módulos de `crates/users/src/adapters/postgres_repository.rs`.
*   ✔️ **Correcto**: El adaptador importa y depende del dominio y de los puertos, implementando las interfaces de estos últimos de forma unidireccional.

### 4. Aislamiento de Dominios por Crates del Workspace
Cada uno de los 8 dominios funcionales clave definidos en el PRD debe estar completamente aislado en su propio crate dentro de la carpeta `crates/`. No se permite una única crate gigante "backend".
*   ❌ **Incorrecto**: Colocar la lógica de mensajería (`chat`) y la lógica de pagos con Stripe (`payments`) dentro del mismo crate `users`.
*   ✔️ **Correcto**: Crear `crates/chat/` y `crates/payments/` como subproyectos independientes en el workspace Cargo con sus propios `Cargo.toml`.

### 5. Crate `api` como Composición Root Única
El crate `api` es el único que puede compilar todos los adaptadores específicos de base de datos e inyectarlos. Ningún otro crate debe enlazar adaptadores de terceros.
*   ❌ **Incorrecto**: El crate `listings` referencia e instancia directamente adaptadores de `payments`.
*   ✔️ **Correcto**: El crate `api` instancia tanto el adaptador de base de datos para `listings` como el de `payments` y los orquesta mediante inyección en el arranque.

### 6. Desacoplamiento de Handlers de Axum con `Arc<dyn Trait>`
Los handlers de Axum en el crate `api` deben depender del trait abstracto en `AppState`, no de structs concretas. Esto facilita cambiar la persistencia a memoria en tests.
*   ❌ **Incorrecto**:
    ```rust
    async fn register(State(state): State<AppState>) {
        state.sqlx_user_repo.find_by_id(1).await; // Acoplamiento rígido
    }
    ```
*   ✔️ **Correcto**:
    ```rust
    async fn register(State(state): State<AppState>) {
        state.user_repo.find_by_id(1).await; // Desacoplado vía Trait
    }
    ```

### 7. Traducir Errores de Infraestructura en la Frontera (Mapeo)
Cualquier error nativo de base de datos (`sqlx::Error`) o de red (errores HTTP de MeiliSearch o Stripe) debe ser interceptado en el adaptador y mapeado a un error específico del dominio antes de salir.
*   ❌ **Incorrecto**: Devolver `Result<User, sqlx::Error>` desde el trait del puerto `UserRepository`.
*   ✔️ **Correcto**: Definir `enum UserError { NotFound, DatabaseError(String) }` en el dominio y mapear el error en el adaptador usando `match` o `.map_err()`.

### 8. Estructura Estándar y Uniforme de Subcarpetas
Todos los crates de dominio deben estructurarse de forma idéntica usando las carpetas `domain/`, `ports/` y `adapters/`.
*   ❌ **Incorrecto**: Tener carpetas llamadas `models/`, `db/`, `controllers/` y `traits/` distribuidas aleatoriamente.
*   ✔️ **Correcto**: Mantener una coherencia visual y técnica rigurosa aplicando la estructura de carpetas `domain/`, `ports/` y `adapters/` en cada crate.

### 9. Grafo de Dependencias de Crates Aclíclico (Sin dependencias circulares)
Las referencias en los archivos `Cargo.toml` entre los crates del workspace deben evitar bucles.
*   ❌ **Incorrecto**: Crate `listings` depende en su `Cargo.toml` de `users`, y a su vez `users` depende de `listings`.
*   ✔️ **Correcto**: Mantener dependencias unidireccionales. Si `listings` requiere validar que el vendedor existe, depende de `users`. Pero `users` no necesita saber nada de `listings`.

### 10. Pureza Absoluta de los Handlers de Axum
Los handlers deben ser controladores ultraligeros. Solo deben validar el payload HTTP, llamar a los métodos del puerto correspondiente y retornar el código de estado HTTP adecuado.
*   ❌ **Incorrecto**: Escribir lógica de hashing de contraseñas o validación de reglas de negocio complejas dentro de un handler de Axum en `crates/api/src/handlers/auth.rs`.
*   ✔️ **Correcto**: Delegar toda la validación e inicio de sesión a un servicio de dominio en `crates/users/` y simplemente responder en base al resultado devuelto.

### 11. Separación de DTOs HTTP vs Entidades de Dominio
Los payloads de entrada y salida JSON (DTOs) deben ser structs independientes de las entidades del dominio para evitar que cambios de formato de API rompan el negocio.
*   ❌ **Incorrecto**: Utilizar la entidad `User` como payload directo en `axum::Json<User>` para registrar un nuevo usuario.
*   ✔️ **Correcto**: Definir `struct RegisterPayload` con campos necesarios (`email`, `password`) en `api::dto` y construir una entidad limpia de dominio `User` a partir de ella.

### 12. Aislamiento Estricto de Macros SQLx (`query!`) en Adaptadores
La compilación estricta y segura de SQLx debe ocurrir únicamente dentro de los archivos ubicados en la carpeta `adapters/` de cada crate.
*   ❌ **Incorrecto**: Colocar llamadas a `sqlx::query!` en archivos dentro de `crates/users/src/domain/` o en `crates/api/src/handlers/`.
*   ✔️ **Correcto**: Restringir el uso de queries SQLx a los archivos correspondientes en `adapters/` (ej: `postgres_repository.rs`), manteniendo el resto del código totalmente inmune al almacenamiento físico.

---

## 🔄 Matriz de Dependencias entre Crates (Correcto vs Incorrecto)

El agente debe revisar y validar la correcta configuración del sistema de dependencias del workspace Cargo en base a este mapa:

| Crate Origen | Crate Destino (Válido) | Crate Destino (INVÁLIDO) | Razón |
| :--- | :--- | :--- | :--- |
| `api` | `users`, `listings`, `payments`, etc. | Ninguno | `api` es el Composición Root y debe poder orquestar todo el sistema. |
| `users` | Ninguno | `api`, `sqlx`, `axum` | El dominio de usuarios debe permanecer puro y desacoplado de la web e infra. |
| `listings` | `users` | `api`, `search` | Un anuncio puede validar la existencia del creador (`users`), pero no conoce la indexación directa (`search`). |
| `search` | `listings` | `api`, `sqlx` | `search` indexa entidades de anuncios, pero no debe depender directamente de la base de datos SQL global. |
| `payments` | `users` | `api`, `stripe` (en dominio) | Los puertos de pagos pueden usar tipos de usuarios, pero el SDK de Stripe es un adaptador externo. |
