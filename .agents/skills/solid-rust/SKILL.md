---
name: solid-rust
description: Directrices de arquitectura, mejores prácticas y patrones de codificación para aplicar los principios SOLID en la separación por traits y la inyección de dependencias en el backend de Rust para Nebripop. Utiliza esta skill siempre que vayas a diseñar interfaces (traits), separar responsabilidades entre servicios, implementar repositorios de base de datos o definir flujos desacoplados de infraestructura.
---

# SOLID Principles in Rust — Nebripop Backend

Esta skill define las mejores prácticas y patrones de diseño de software para aplicar los principios **SOLID** en el backend en Rust de **Nebripop**. A través de la arquitectura hexagonal por crates establecida en el proyecto, utilizaremos el sistema de tipos, traits y el compilador de Rust para forzar el desacoplamiento entre las reglas de negocio (casos de uso) y los detalles de infraestructura (PostgreSQL/SQLx, Stripe y Cloudinary).

---

## 1. SRP: Single Responsibility Principle (Principio de Responsabilidad Única)

### Contexto en Rust y Nebripop
Un struct o módulo debe tener **una sola responsabilidad y un único motivo de cambio**. En Nebripop, la lógica de negocio (casos de uso), la persistencia en base de datos (repositorios) y los clientes externos (hashing, emails, Stripe) deben estar encapsulados en componentes distintos.

### ❌ Violación del SRP
El struct `UserRepository` asume tres tareas distintas: ejecutar consultas SQL, hashear contraseñas usando Argon2id y realizar llamadas SMTP para enviar correos de bienvenida. Si cambia el proveedor de correo o el algoritmo de hashing, este struct de persistencia tiene que modificarse.

```rust
// CRÍTICA VIOLACIÓN DEL SRP
pub struct UserRepository {
    db: sqlx::PgPool,
}

impl UserRepository {
    pub async fn create_user(&self, email: &str, raw_pw: &str) -> Result<User, sqlx::Error> {
        // 1. Lógica de hashing acoplada
        let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());
        let pw_hash = argon2::Argon2::default()
            .hash_password(raw_pw.as_bytes(), &salt)?
            .to_string();

        // 2. Persistencia en base de datos
        let user = sqlx::query_as!(User, "INSERT INTO users...", email, pw_hash)
            .fetch_one(&self.db)
            .await?;

        // 3. Llamada externa SMTP inyectada
        smtp::send_welcome_email(email).await;

        Ok(user)
    }
}
```

### ✅ Implementación Correcta
Separar las responsabilidades en componentes específicos e independientes. El caso de uso coordina a los tres colaboradores.

```rust
// 1. Repositorio enfocado en exclusiva a la persistencia en PostgreSQL
pub struct UserRepository {
    db: sqlx::PgPool,
}
impl UserRepository {
    pub async fn save_user(&self, email: &str, password_hash: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as!(User, "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING *", email, password_hash)
            .fetch_one(&self.db)
            .await
    }
}

// 2. Componente de infraestructura dedicado al hashing
pub struct PasswordHasher;
impl PasswordHasher {
    pub fn hash(&self, password: &str) -> Result<String, argon2::password_hash::Error> { ... }
}

// 3. Componente de infraestructura para comunicaciones
pub struct EmailService;
impl EmailService {
    pub async fn send_welcome(&self, email: &str) { ... }
}

// 4. El caso de uso orquesta la lógica de negocio sin mezclar implementaciones
pub async fn register_user_usecase(
    email: &str,
    raw_pw: &str,
    repo: &UserRepository,
    hasher: &PasswordHasher,
    notifier: &EmailService,
) -> Result<User, DomainError> {
    let hash = hasher.hash(raw_pw)?;
    let user = repo.save_user(email, &hash).await?;
    notifier.send_welcome(email).await;
    Ok(user)
}
```

### Cómo Ayuda el Compilador de Rust a Mantener el SRP
Rust previene el acoplamiento involuntario de dependencias a través de su **sistema estricto de visibilidad de módulos (`pub(crate)`) y crates**. Al encapsular la lógica de hashing y correo en submódulos separados, el compilador emitirá advertencias e impedirá compilar si intentas importar crates masivos de infraestructura (como `lettre` para SMTP) en el crate de persistencia puro de base de datos.

---

## 2. OCP: Open/Closed Principle (Principio de Abierto/Cerrado)

### Contexto en Rust y Nebripop
Las entidades de software deben estar **abiertas para la extensión pero cerradas para la modificación**. En Rust, esto se logra mediante el uso de **Traits** polimórficos. En lugar de modificar el caso de uso principal cada vez que añadimos un canal de comunicación (ej. avisar al vendedor de un nuevo mensaje por email, por WebSocket en tiempo real o por push en el móvil), encapsulamos el comportamiento en una abstracción.

### ❌ Violación del OCP
El caso de uso del chat depende de un bloque `match` masivo que inspecciona un enum. Cada vez que queramos añadir un canal nuevo, nos vemos obligados a modificar el código central del caso de uso.

```rust
pub enum NotificationChannel {
    Email,
    WebSocket,
    Push,
}

// Si añadimos un nuevo canal (ej. SMS), tenemos que alterar este código de negocio central
pub async fn notify_user_violation(user_id: uuid::Uuid, channel: NotificationChannel, msg: &str) {
    match channel {
        NotificationChannel::Email => smtp_client::send_email(user_id, msg).await,
        NotificationChannel::WebSocket => ws_server::send_direct_message(user_id, msg).await,
        NotificationChannel::Push => firebase_client::send_push(user_id, msg).await,
    }
}
```

### ✅ Implementación Correcta
Definir un **Trait** abstracto `NotificationSender` que encapsule el comportamiento. La lógica de negocio solo sabe invocar al método `send`, cerrando la función a modificaciones.

```rust
use async_trait::async_trait;

#[async_trait]
pub trait NotificationSender: Send + Sync {
    async fn send(&self, recipient_id: uuid::Uuid, message: &str) -> Result<(), NotificationError>;
}

// Implementación 1: Email
pub struct EmailSender;
#[async_trait]
impl NotificationSender for EmailSender {
    async fn send(&self, recipient_id: uuid::Uuid, message: &str) -> Result<(), NotificationError> { ... }
}

// Implementación 2: WebSocket
pub struct WebSocketSender;
#[async_trait]
impl NotificationSender for WebSocketSender {
    async fn send(&self, recipient_id: uuid::Uuid, message: &str) -> Result<(), NotificationError> { ... }
}

// El caso de uso está cerrado a modificaciones. Acepta cualquier struct que implemente el Trait
pub async fn notify_user_clean(
    recipient_id: uuid::Uuid,
    message: &str,
    sender: &dyn NotificationSender, // Inyección por dynamic dispatch o genéricos
) -> Result<(), NotificationError> {
    sender.send(recipient_id, message).await
}
```

### Cómo Ayuda el Compilador de Rust a Mantener el OCP
El compilador fuerza el cumplimiento del principio garantizando que **cualquier nueva implementación del trait cumpla estrictamente con el contrato estipulado**. Si intentas añadir una implementación de `NotificationSender` sin definir el método `send` con la firma exacta y su retorno `Result`, el compilador abortará el build de inmediato, manteniendo la integridad del sistema.

---

## 3. LSP: Liskov Substitution Principle (Principio de Sustitución de Liskov)

### Contexto en Rust y Nebripop
Los subtipos (o structs que implementan un Trait en Rust) deben ser **sustituibles por su tipo base sin alterar la corrección del programa**. En Nebripop, cualquier implementación de un repositorio de persistencia (ej. un mock en memoria para tests de integración y el repositorio SQLx PostgreSQL para producción) debe comportarse de forma idéntica ante los contratos definidos.

### ❌ Violación del LSP
La implementación Mock de `ListingRepository` viola el principio porque, si el anuncio no se encuentra en memoria, arroja un pánico (`panic!`) en lugar de retornar la firma pactada de `Ok(None)`. El código de negocio que llama al repositorio explotará si sustituimos la base de datos real por el mock en los tests.

```rust
// Trait oficial de persistencia
#[async_trait]
pub trait ListingRepository {
    async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<Listing>, RepositoryError>;
}

// Implementación incorrecta en memoria para tests
pub struct InMemoryListingRepository {
    data: std::sync::Mutex<HashMap<uuid::Uuid, Listing>>,
}

#[async_trait]
impl ListingRepository for InMemoryListingRepository {
    async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<Listing>, RepositoryError> {
        let map = self.data.lock().unwrap();
        
        // VIOLACIÓN LSP: Causa panic si no existe, alterando la firma lógica y el comportamiento esperado!
        let listing = map.get(&id).expect("¡Anuncio no encontrado en test!"); 
        
        Ok(Some(listing.clone()))
    }
}
```

### ✅ Implementación Correcta
Asegurar que todas las implementaciones respetan las invariantes semánticas del Trait, retornando `Ok(None)` de forma segura sin disparar panics.

```rust
#[async_trait]
impl ListingRepository for InMemoryListingRepository {
    async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<Listing>, RepositoryError> {
        let map = self.data.lock().unwrap();
        // Comportamiento idéntico al de base de datos
        Ok(map.get(&id).cloned())
    }
}
```

### Cómo Ayuda el Compilador de Rust a Mantener el LSP
Rust no posee herencia tradicional de clases, lo que **elimina la mayoría de los problemas de acoplamiento de LSP**. En su lugar, el compilador fuerza el cumplimiento estricto del LSP mediante los tipos de retorno definidos en el Trait. Ninguna implementación puede retornar un tipo diferente, obligando al programador a manejar exactamente las mismas salidas estructuradas en todas las implementaciones.

---

## 4. ISP: Interface Segregation Principle (Principio de Segregación de Interfaces)

### Contexto en Rust y Nebripop
**Los clientes no deben ser forzados a depender de interfaces o traits que no utilizan**. En lugar de crear traits monolíticos con decenas de métodos para la base de datos de Nebripop, debemos segregarlos en traits pequeños y altamente cohesivos (ej. separar lectura de escritura).

### ❌ Violación del ISP
El trait `UserRepository` es monolítico. Contiene 10 métodos que combinan lectura de perfil, escritura, eliminación y consultas específicas. Si un caso de uso de chat solo necesita validar que el ID del comprador existe en el sistema (Lectura), es obligado a depender de métodos masivos de borrado y modificación de contraseñas.

```rust
// VIOLACIÓN DEL ISP: Trait masivo no segregado
#[async_trait]
pub trait UserRepository {
    async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<User>, sqlx::Error>;
    async fn save(&self, user: &User) -> Result<(), sqlx::Error>;
    async fn update_password(&self, id: uuid::Uuid, hash: &str) -> Result<(), sqlx::Error>;
    async fn update_avatar(&self, id: uuid::Uuid, url: &str) -> Result<(), sqlx::Error>;
    async fn delete_user(&self, id: uuid::Uuid) -> Result<(), sqlx::Error>;
}
```

### ✅ Implementación Correcta
Segregar el trait en piezas pequeñas y enfocadas. Un struct de repositorio real (`PostgresUserRepository`) puede implementar múltiples traits específicos a la vez.

```rust
// Traits pequeños, específicos y cohesivos
#[async_trait]
pub trait UserReader {
    async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<User>, sqlx::Error>;
}

#[async_trait]
pub trait UserProfileWriter {
    async fn update_avatar(&self, id: uuid::Uuid, url: &str) -> Result<(), sqlx::Error>;
    async fn update_bio(&self, id: uuid::Uuid, bio: &str) -> Result<(), sqlx::Error>;
}

#[async_trait]
pub trait UserCredentialsWriter {
    async fn update_password(&self, id: uuid::Uuid, hash: &str) -> Result<(), sqlx::Error>;
}

// El adaptador de persistencia en PostgreSQL implementa todos
pub struct PostgresUserRepository {
    db: sqlx::PgPool,
}
#[async_trait]
impl UserReader for PostgresUserRepository { ... }
#[async_trait]
impl UserProfileWriter for PostgresUserRepository { ... }
```

### Cómo Ayuda el Compilador de Rust a Mantener el ISP
Rust permite combinar interfaces segregadas fácilmente en tiempo de compilación usando los **Trait Bounds** múltiples (`impl Trait1 + Trait2`). Si un caso de uso de administración requiere tanto lectura como modificación del perfil, la firma de la función simplemente los combina de forma elegante:

```rust
pub async fn ban_user_usecase<T>(
    user_id: uuid::Uuid, 
    repo: &T
) -> Result<(), DomainError> 
where 
    T: UserReader + UserProfileWriter // Combinación a la carta sin forzar herencia
{
    // ...
}
```

---

## 5. DIP: Dependency Inversion Principle (Principio de Inversión de Dependencias)

### Contexto en Rust y Nebripop
**Los módulos de alto nivel (casos de uso de Nebripop) no deben depender de módulos de bajo nivel (base de datos SQLx, pasarela Stripe). Ambos deben depender de abstracciones (Traits)**. Esto desacopla las reglas de negocio de los detalles tecnológicos específicos de infraestructura.

### ❌ Violación del DIP
El caso de uso para procesar una compraventa depende directamente de los structs concretos de infraestructura de base de datos (`sqlx::PgPool`) y del SDK cliente de Stripe (`stripe::Client`). Si quisiéramos cambiar Stripe por PayPal o testear la lógica de cobro sin tocar la BD real, no podríamos hacerlo.

```rust
// VIOLACIÓN DEL DIP: Acoplamiento a implementaciones de infraestructura
pub async fn process_purchase_usecase(
    listing_id: uuid::Uuid,
    buyer_id: uuid::Uuid,
    db_pool: &sqlx::PgPool,          // Concreto de SQLx (Bajo nivel)
    stripe_client: &stripe::Client,  // Concreto de Stripe SDK (Bajo nivel)
) -> Result<(), AppError> {
    // ... lógica de cobro e inserciones ...
}
```

### ✅ Implementación Correcta
Definir traits para la base de datos y la pasarela de pagos. El caso de uso se comunica única y exclusivamente con estas abstracciones.

```rust
// 1. Abstracción de Persistencia
#[async_trait]
pub trait ListingRepository: Send + Sync {
    async fn get_active_listing(&self, id: uuid::Uuid) -> Result<Option<Listing>, RepositoryError>;
    async fn mark_as_sold(&self, id: uuid::Uuid) -> Result<(), RepositoryError>;
}

// 2. Abstracción de Pagos
#[async_trait]
pub trait PaymentGateway: Send + Sync {
    async fn charge(&self, amount: rust_decimal::Decimal, description: &str) -> Result<ChargeReceipt, PaymentError>;
}

// 3. Caso de Uso desacoplado usando Generics (Static Dispatch / Monomorfización a coste cero)
pub async fn process_purchase_usecase<R, P>(
    listing_id: uuid::Uuid,
    buyer_id: uuid::Uuid,
    repo: &R,
    gateway: &P,
) -> Result<(), DomainError> 
where 
    R: ListingRepository,
    P: PaymentGateway,
{
    let listing = repo.get_active_listing(listing_id).await?
        .ok_or(DomainError::NotFound)?;

    // Cobrar al comprador delegando en el Trait
    let receipt = gateway.charge(listing.price, &listing.title).await?;

    // Marcar en BD delegando en el Trait
    repo.mark_as_sold(listing_id).await?;

    Ok(())
}
```

### Cómo Ayuda el Compilador de Rust a Mantener el DIP
El compilador de Rust procesa los genéricos aplicando **Monomorfización**. En lugar de incurrir en penalizaciones de rendimiento en tiempo de ejecución (típico en lenguajes que usan interfaces dinámicas con punteros virtuales o reflexión), el compilador de Rust genera copias de la función en tiempo de compilación con las llamadas resueltas estáticamente para cada struct concreto (`PostgresListingRepository`, `StripePaymentGateway`). **Obtenemos un desacoplamiento arquitectónico perfecto al coste de rendimiento de una llamada directa de función (Abstracción a coste cero).**

---

## 6. Las 10 Reglas Críticas de SOLID para Nebripop

1. **Aislamiento en Módulos de Persistencia (SRP)**: Los structs encargados de base de datos (`PostgresUserRepository`) no deben contener lógica criptográfica ni inicializar peticiones externas de APIs (como Cloudinary o Stripe).
2. **Definición de Canales mediante Traits (OCP)**: Cualquier extensión de comunicación (notificaciones, sistemas de envío, WebSockets) debe declararse mediante un trait abstracto para poder incorporar futuros canales de forma transparente.
3. **Respeto a las Invariantes (LSP)**: Está terminantemente prohibido que una implementación mock o en memoria de un trait de persistencia lance un pánico (`panic!`) o altere las firmas de salida en caso de ausencia de registros. Deben retornar el mismo `Result<Option<T>>` que el adaptador SQLx.
4. **Segregación de Operaciones Lectura/Escritura (ISP)**: Evita construir traits de repositorio monolíticos. Segrega el acceso a datos en traits específicos (ej: `ListingReader` para el feed de búsqueda y `ListingWriter` para la creación y edición de anuncios).
5. **Dependencia de Abstracciones en Casos de Uso (DIP)**: Los casos de uso (`usecases`) de Nebripop nunca deben importar tipos concretos de `sqlx` (como `PgPool` o `sqlx::Error`) ni clientes de servicios externos directos. Deben operar a través de traits genéricos.
6. **Uso de Genéricos para Static Dispatch**: Prioriza el uso de parámetros genéricos y bounds de traits (`where T: Trait`) en los casos de uso para que el compilador resuelva las implementaciones estáticamente en tiempo de compilación (abstracción a coste cero).
7. **Dynamic Dispatch Solo Si es Necesario**: Utiliza punteros dinámicos (`&dyn Trait` o `Box<dyn Trait>`) únicamente cuando requieras almacenar colecciones heterogéneas de structs que implementan el mismo trait en tiempo de ejecución (ej. un vector de notificadores).
8. **Encapsulamiento Criptográfico (SRP)**: La lógica de Argon2id debe residir en un componente independiente (`PasswordHasher`). Las clases de servicio que interactúan con registros nunca deben manipular saltings ni hashes criptográficos crudos de forma directa.
9. **Cierre de Reglas de Negocio Centrales**: Los flujos core de transacciones y cobros (`US-17`, `US-18`) deben estar cerrados a modificaciones tecnológicas específicas; cualquier cambio de pasarela debe realizarse mediante implementaciones secundarias de traits existentes.
10. **Aislamiento Absoluto de Criterios de Validación (ISP)**: Las interfaces y traits no deben obligar a structs que las implementan a validar campos que escapan a su lógica operativa básica. Limita los traits a tareas cohesivas y de un único propósito.
