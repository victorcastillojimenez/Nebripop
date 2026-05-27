# Arquitectura Técnica — Nebripop

**Versión:** v1.0
**Fecha:** 27 de mayo de 2026
**Autor:** architect-agent
**Fuente de verdad:** Este documento es la referencia técnica única (Single Source of Truth) del proyecto Nebripop.

---

## 1. Diagramas C4

### 1.1 Nivel Contexto — Sistema y actores externos

```mermaid
graph TD
    Anon([Usuario Anónimo])
    Buyer([Usuario Comprador])
    Seller([Usuario Vendedor])
    Admin([Administrador])

    Anon -->|HTTPS| NEB[Nebripop - Marketplace C2C]
    Buyer -->|HTTPS / WSS| NEB
    Seller -->|HTTPS / WSS| NEB
    Admin -->|HTTPS| NEB

    NEB -->|HTTPS| STRIPE[Stripe API - Pagos]
    NEB -->|HTTPS| CLOUD[Cloudinary API - Imágenes]
    NEB -->|HTTPS| MEILI[MeiliSearch - Búsqueda full-text]
    NEB -->|TCP| PG[(PostgreSQL 15 - Base de datos)]
```

### 1.2 Nivel Contenedor — Componentes desplegables

```mermaid
graph TD
    Browser([Navegador del usuario])

    Browser -->|HTTPS / WSS| AXUM[Axum Backend - Puerto 8080]

    subgraph Nebripop Server
        AXUM -->|Renderiza| ASKAMA[Askama Templates SSR + TailwindCSS CDN + JS Vanilla]
        AXUM -->|SQLx pool| PG[(PostgreSQL 15)]
        AXUM -->|HTTP SDK| MEILI[MeiliSearch 1.x]
    end

    AXUM -->|HTTPS stripe-rust| STRIPE[Stripe API]
    AXUM -->|HTTPS SDK| CLOUD[Cloudinary API]

    STRIPE -.->|Webhook POST /payments/webhook| AXUM
```

### 1.3 Nivel Componente — Crates del workspace Cargo

```mermaid
graph TD
    subgraph Cargo Workspace
        API[crate: api - Orquestador Axum]
        USERS[crate: users - Auth y perfiles]
        LISTINGS[crate: listings - Anuncios e imágenes]
        SEARCH[crate: search - Indexación y búsqueda]
        CHAT[crate: chat - Mensajería WebSocket]
        PAYMENTS[crate: payments - Pagos Stripe]
        RATINGS[crate: ratings - Valoraciones]
        FAVORITES[crate: favorites - Favoritos]
        GEO[crate: geo - Geolocalización Haversine]
        COMMON[crate: common - Tipos compartidos]
    end

    API --> USERS
    API --> LISTINGS
    API --> SEARCH
    API --> CHAT
    API --> PAYMENTS
    API --> RATINGS
    API --> FAVORITES
    API --> GEO

    LISTINGS --> USERS
    LISTINGS --> GEO
    LISTINGS --> COMMON
    CHAT --> USERS
    CHAT --> LISTINGS
    CHAT --> COMMON
    PAYMENTS --> LISTINGS
    PAYMENTS --> COMMON
    RATINGS --> USERS
    RATINGS --> COMMON
    FAVORITES --> LISTINGS
    FAVORITES --> COMMON
    SEARCH --> LISTINGS
    SEARCH --> COMMON
    GEO --> COMMON
    USERS --> COMMON

    LISTINGS -.->|adapter| CLOUD_API[Cloudinary]
    SEARCH -.->|adapter| MEILI_API[MeiliSearch]
    PAYMENTS -.->|adapter| STRIPE_API[Stripe]

    API -.->|SQLx pool| PG_DB[(PostgreSQL)]
```

---

## 2. Decisiones de Arquitectura (ADRs)

### ADR-001: Rust + Axum + Tokio como stack backend

**Estatus**: Aceptado
**Fecha**: 2026-05-27

**Contexto**
Nebripop requiere un backend con alta concurrencia para manejar WebSockets de chat, peticiones REST simultáneas y webhooks de Stripe. El equipo necesita un framework que ofrezca rendimiento nativo, seguridad de memoria sin garbage collector y un ecosistema maduro para APIs HTTP asíncronas. El requisito no funcional del PRD exige P95 < 200ms en endpoints REST y < 100ms en latencia de WebSocket.

**Decisión**
Adoptar Rust como lenguaje principal del backend, Axum 0.7.x como framework web HTTP y WebSocket, y Tokio 1.x como runtime asíncrono. Axum ofrece extractores tipados, integración nativa con Tower middleware, y soporte de WebSockets integrado mediante `axum::extract::ws`. Se descartaron alternativas como Actix Web (API menos ergonómica y macros implícitas) y frameworks en otros lenguajes (Node.js/Express, Go/Gin) por no cumplir simultáneamente los requisitos de rendimiento, seguridad de tipos y validación en tiempo de compilación.

**Consecuencias**
- ✅ Rendimiento nativo sin garbage collector: cumple P95 < 200ms con margen
- ✅ Seguridad de memoria garantizada en compilación (borrow checker)
- ✅ Soporte nativo de WebSockets en Axum sin dependencias adicionales
- ✅ Ecosistema async maduro con Tokio (spawn, select, channels)
- ⚠️ Curva de aprendizaje elevada del sistema de tipos y lifetimes para generación con IA
- ⚠️ Tiempos de compilación más largos que lenguajes interpretados (mitigado con compilación incremental por crate)

---

### ADR-002: Askama templates + TailwindCSS CDN + JavaScript vanilla sobre Leptos/WASM

**Estatus**: Aceptado
**Fecha**: 2026-05-27

**Contexto**
El frontend de Nebripop debe renderizar páginas HTML completas del lado del servidor para optimizar SEO y First Contentful Paint (< 1.5s según PRD). El equipo tiene 1 semana de desarrollo y necesita una solución de frontend que no añada complejidad de build (bundlers, WASM, hydration). Se evaluaron Leptos (framework Rust con WASM), Yew (WASM) y la combinación Askama + TailwindCSS CDN + JS vanilla.

**Decisión**
Usar Askama 0.12.x para templates HTML compilados y validados en tiempo de compilación, TailwindCSS via CDN para estilos responsive sin build step, y JavaScript vanilla para interactividad mínima (formularios, WebSocket client, toggles de favoritos). Askama genera HTML estático embebido en el binario, eliminando la necesidad de un servidor de archivos estáticos separado. Se descarta Leptos/WASM por complejidad de hydration, mayor tamaño de bundle y tiempo de desarrollo incompatible con 1 semana.

**Consecuencias**
- ✅ Templates validados en compilación: errores de tipado detectados antes de ejecutar
- ✅ Zero build step adicional para frontend (no webpack, no Vite, no trunk)
- ✅ First Contentful Paint óptimo: HTML completo en primera respuesta
- ✅ TailwindCSS CDN permite diseño responsive sin configuración local
- ⚠️ Interactividad limitada a JavaScript vanilla (sin estado reactivo en cliente)
- ⚠️ Cada cambio de template requiere recompilación del binario Rust

---

### ADR-003: PostgreSQL + SQLx sobre ORM (Diesel, SeaORM)

**Estatus**: Aceptado
**Fecha**: 2026-05-27

**Contexto**
Nebripop necesita persistencia relacional para 8 entidades con relaciones complejas (usuarios, anuncios, conversaciones, transacciones, valoraciones, favoritos). Se requiere validación de queries SQL en compilación para eliminar errores de runtime, soporte nativo de UUID, TIMESTAMPTZ y NUMERIC, y migraciones versionadas. Se evaluaron Diesel (ORM con DSL propio), SeaORM (ORM async) y SQLx (driver SQL directo con validación en compilación).

**Decisión**
Adoptar PostgreSQL 15+ como motor de base de datos y SQLx 0.7.x como driver asíncrono con validación de queries en compilación. SQLx verifica cada `query!` y `query_as!` contra el esquema real de la base de datos durante `cargo build`, detectando columnas inexistentes, tipos incompatibles y errores de SQL antes del despliegue. Se descartan ORMs por la abstracción innecesaria que imponen sobre queries SQL directas y la pérdida de control fino sobre índices y optimización.

**Consecuencias**
- ✅ Queries SQL validadas contra el esquema real en compilación (zero errores SQL en runtime)
- ✅ Control total sobre queries: optimización manual de JOINs, índices y CTEs
- ✅ Soporte nativo de tipos PostgreSQL: UUID, TIMESTAMPTZ, NUMERIC, FLOAT8, BOOLEAN
- ✅ Migraciones versionadas con `sqlx migrate run` en directorio `/migrations`
- ⚠️ Requiere base de datos accesible durante compilación (solucionable con `sqlx prepare` offline)
- ⚠️ No genera automáticamente structs de entidad: cada modelo se define manualmente

---

### ADR-004: JWT (jsonwebtoken) + Argon2id para autenticación

**Estatus**: Aceptado
**Fecha**: 2026-05-27

**Contexto**
El PRD exige autenticación stateless con tokens que expiren en 24h, hashing de contraseñas con parámetros OWASP (m=19456, t=2, p=1), y validación en cada request protegido mediante middleware de Axum. Se necesita que el sistema funcione sin almacenamiento de sesiones en servidor (sin Redis obligatorio). Se evaluaron sesiones basadas en cookies con almacenamiento servidor, OAuth2 con proveedores externos, y JWT stateless con hashing Argon2id.

**Decisión**
Implementar autenticación mediante JWT con algoritmo HS256 usando el crate `jsonwebtoken` 9.x para generación y validación de tokens, y `argon2` 0.5.x para hashing de contraseñas con Argon2id. El token JWT contiene `user_id` (UUID), `role` (user/admin) y `exp` (expiración 24h). Un middleware de Axum extrae el header `Authorization: Bearer <token>`, valida firma y expiración, e inyecta `Extension(CurrentUser)` en los handlers protegidos. Se descarta OAuth2 externo por complejidad innecesaria para un MVP académico, y sesiones con estado por requerir Redis.

**Consecuencias**
- ✅ Stateless: no requiere almacenamiento de sesiones en servidor ni Redis
- ✅ Argon2id con parámetros OWASP: resistente a ataques de fuerza bruta y side-channel
- ✅ Middleware reutilizable en Axum: una única capa valida todos los endpoints protegidos
- ✅ Token portable: permite autenticar tanto requests HTTP como handshakes WebSocket
- ⚠️ No hay revocación inmediata de tokens (token válido hasta expiración natural de 24h)
- ⚠️ JWT_SECRET debe protegerse estrictamente: si se filtra, todos los tokens son comprometidos

---

### ADR-005: MeiliSearch para búsqueda full-text

**Estatus**: Aceptado
**Fecha**: 2026-05-27

**Contexto**
El PRD exige búsqueda por texto libre con resultados en menos de 300ms (US-07), filtros combinables por categoría, rango de precio y distancia (US-08), y tolerancia a errores tipográficos. PostgreSQL con `ILIKE` no ofrece ranking de relevancia ni typo-tolerance nativo. Se evaluaron Elasticsearch (pesado, JVM, complejo para 1 semana), PostgreSQL full-text search con `tsvector` (limitado en typo-tolerance), y MeiliSearch (ligero, API REST, typo-tolerant por defecto).

**Decisión**
Adoptar MeiliSearch como motor de búsqueda externo usando el crate `meilisearch-sdk` 0.27.x. Los anuncios se indexan asincrónicamente en un índice `listings_index` cada vez que se crean, modifican o eliminan. Las búsquedas del usuario se dirigen a MeiliSearch con filtros de categoría, precio y geolocalización. Si MeiliSearch no está disponible, el backend ejecuta un fallback automático a PostgreSQL con `ILIKE` y filtros `WHERE`.

**Consecuencias**
- ✅ Búsqueda sub-50ms con ranking de relevancia y typo-tolerance automática
- ✅ Filtros faceteados nativos (categoría, precio, geolocalización) sin queries SQL complejas
- ✅ API REST simple con SDK Rust oficial disponible
- ✅ Fallback a PostgreSQL ILIKE garantiza disponibilidad aunque MeiliSearch caiga
- ⚠️ Servicio externo adicional que mantener desplegado y sincronizado
- ⚠️ Indexación asíncrona implica ventana de consistencia eventual (< 5 segundos según PRD)

---

### ADR-006: Stripe para pagos

**Estatus**: Aceptado
**Fecha**: 2026-05-27

**Contexto**
El PRD exige pagos reales con tarjeta (US-17, US-18) con cero datos de tarjeta almacenados en la base de datos del sistema (PCI-DSS compliance). Se necesita crear PaymentIntents, redirigir a Stripe Checkout, y procesar webhooks asincrónicos para confirmar el pago y actualizar el estado de la transacción. Se evaluaron PayPal (API más compleja, peor DX), Stripe (API clara, SDK Rust oficial, modo test gratuito), y procesamiento manual (inviable por compliance).

**Decisión**
Usar Stripe como pasarela de pagos mediante el crate `stripe-rust` 0.34.x. El flujo implementa: (1) el comprador solicita pago via `POST /payments/intent`, (2) el backend crea un PaymentIntent con monto y metadata, (3) el cliente redirige a Stripe Checkout, (4) Stripe notifica via webhook a `POST /payments/webhook`, (5) el backend valida la firma `Stripe-Signature`, actualiza la transacción a `paid` y el listing a `sold`. Se opera en modo test durante desarrollo y se conmuta a modo producción para demo.

**Consecuencias**
- ✅ Cero datos de tarjeta en nuestra BD: compliance PCI-DSS delegado 100% a Stripe
- ✅ SDK Rust oficial (`stripe-rust`) con tipos seguros para PaymentIntent y webhooks
- ✅ Modo test gratuito: permite desarrollo completo sin costes reales
- ✅ Webhooks asíncronos permiten actualización fiable del estado de transacción
- ⚠️ Dependencia de servicio externo: si Stripe cae, los pagos se bloquean
- ⚠️ Validación criptográfica de webhooks añade complejidad al handler

---

### ADR-007: Cloudinary para gestión de imágenes

**Estatus**: Aceptado
**Fecha**: 2026-05-27

**Contexto**
Cada anuncio en Nebripop puede tener múltiples imágenes (tabla `listing_images`). Se necesita almacenamiento escalable, redimensionamiento dinámico (thumbnails, previews) y entrega via CDN para optimizar tiempos de carga. Se evaluaron: almacenamiento local en disco (`/static/uploads/`), Amazon S3 + CloudFront (requiere cuenta AWS y configuración IAM compleja), y Cloudinary (API REST, transformaciones on-the-fly, plan gratuito suficiente para MVP).

**Decisión**
Adoptar Cloudinary como servicio de almacenamiento y transformación de imágenes. Las imágenes se suben via HTTPS desde el crate `listings` al crear o editar anuncios. La URL resultante de Cloudinary se persiste en la tabla `listing_images`. Las transformaciones (resize, crop, formato WebP) se aplican via parámetros en la URL de Cloudinary sin procesamiento local. Si Cloudinary no está disponible, el fallback almacena imágenes localmente en `/static/uploads/` con servido estático desde Axum.

**Consecuencias**
- ✅ CDN global de Cloudinary reduce latencia de carga de imágenes para usuarios
- ✅ Transformaciones dinámicas sin procesamiento local (thumbnails, WebP, crop)
- ✅ Plan gratuito cubre el volumen del MVP (25 créditos/mes, ~25GB almacenamiento)
- ✅ Fallback a almacenamiento local garantiza funcionalidad sin servicio externo
- ⚠️ Dependencia de servicio externo para funcionalidad core (imágenes de anuncios)
- ⚠️ Requiere gestión de API keys y upload presets en la configuración

---

### ADR-008: WebSockets con axum::extract::ws para chat en tiempo real

**Estatus**: Aceptado
**Fecha**: 2026-05-27

**Contexto**
El PRD exige mensajería en tiempo real con latencia < 100ms entre envío y renderizado (US-11). Los mensajes deben persistirse en PostgreSQL y entregarse al destinatario si está conectado, o almacenarse para lectura posterior. Se evaluaron: polling HTTP periódico (latencia alta, tráfico innecesario), Server-Sent Events (unidireccional, no apto para chat), y WebSockets bidireccionales. Para la implementación en Rust se evaluaron `tokio-tungstenite` (bajo nivel) y `axum::extract::ws` (integrado en Axum).

**Decisión**
Implementar chat en tiempo real usando `axum::extract::ws` que proporciona WebSockets integrados en el framework Axum sin dependencias adicionales. El handshake WebSocket requiere un token JWT válido como query parameter (`?token=<jwt>`). Cada conexión activa se registra en un `DashMap<Uuid, Sender>` en el `AppState`, aprovechando la concurrencia lock-free de DashMap para evitar deadlocks y mejorar el rendimiento bajo alta carga. Al recibir un mensaje, el handler lo persiste en PostgreSQL y lo reenvía al destinatario si está conectado. Como fallback, se implementa `GET /chat/:id/messages?since=<timestamp>` para recuperar mensajes via HTTP polling.

**Consecuencias**
- ✅ Latencia sub-100ms: comunicación bidireccional persistente sin overhead HTTP por mensaje
- ✅ Integración nativa con Axum: reutiliza el mismo servidor, middleware y AppState
- ✅ Persistencia dual: cada mensaje se guarda en PostgreSQL y se entrega en tiempo real
- ✅ Fallback HTTP polling garantiza funcionalidad si WebSocket no está disponible
- ✅ DashMap ofrece acceso concurrente sin locks globales y sin riesgo de deadlocks
- ⚠️ Sin pub/sub externo (Redis): limitado a una única instancia del servidor en esta versión

## 3. Estructura de Crates del Workspace Cargo

### 3.1 Workspace raíz — Cargo.toml

```toml
[workspace]
members = [
    "crates/api",
    "crates/common",
    "crates/users",
    "crates/listings",
    "crates/search",
    "crates/chat",
    "crates/payments",
    "crates/ratings",
    "crates/favorites",
    "crates/geo",
]
resolver = "2"

[workspace.dependencies]
axum = { version = "0.7", features = ["ws", "multipart"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["postgres", "uuid", "chrono", "runtime-tokio-native-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
tracing = "0.1"
tower-http = { version = "0.5", features = ["cors", "trace", "compression-gzip"] }
jsonwebtoken = "9"
argon2 = "0.5"
validator = { version = "0.18", features = ["derive"] }
rust_decimal = { version = "1", features = ["serde-float"] }
dashmap = "6"
futures-util = "0.3"
askama = { version = "0.12", features = ["with-axum"] }
meilisearch-sdk = "0.27"
```

### 3.2 crate: common

**Responsabilidad**: Tipos compartidos, errores base y utilidades reutilizables por todos los crates.

```
crates/common/src/
├── lib.rs
├── errors.rs        # AppError global con IntoResponse para Axum
├── pagination.rs    # PageRequest { page, per_page }, PageResult<T>
├── ids.rs           # Newtypes: UserId(Uuid), ListingId(Uuid), ChatId(Uuid), PaymentId(Uuid)
└── response.rs      # Formato unificado de respuesta JSON { success, data, error }
```

**Traits expuestos**: Ninguno (solo tipos y utilidades).
**Dependencias**: Ninguna de otros crates del workspace.

---

### 3.3 crate: users

**Responsabilidad**: Autenticación JWT, registro, login, perfiles públicos y extractor AuthUser reutilizable por todos los módulos protegidos.

```
crates/users/src/
├── lib.rs
├── router.rs
├── errors.rs
├── models.rs              # User, UserRole(User/Admin), PublicProfile
├── dtos.rs                # RegisterDto, LoginDto, AuthResponse, PublicProfileDto
├── handlers/
│   ├── mod.rs
│   ├── register.rs        # POST /auth/register
│   ├── login.rs           # POST /auth/login
│   ├── refresh.rs         # POST /auth/refresh
│   ├── logout.rs          # POST /auth/logout
│   └── get_profile.rs     # GET /users/:id
├── usecases/
│   ├── mod.rs
│   ├── register_usecase.rs
│   ├── login_usecase.rs
│   └── refresh_usecase.rs
└── adapters/
    ├── mod.rs
    ├── user_repository.rs
    ├── jwt.rs              # generate_jwt(), verify_jwt(), Claims
    └── password.rs         # hash_password(), verify_password() Argon2id OWASP
```

**Traits expuestos**:
- `UserRepository`: find_by_id, find_by_email, insert, update_last_login
- `AuthService`: generate_token, verify_token

**Dependencias**: common

---

### 3.4 crate: listings

**Responsabilidad**: CRUD completo de anuncios, subida de imágenes a Cloudinary, soft delete y gestión del ciclo de vida.

```
crates/listings/src/
├── lib.rs
├── router.rs
├── errors.rs              # ListingError: NotFound, NotOwner, TooManyImages, InvalidInput
├── models.rs              # Listing, ListingImage, ListingStatus(Active/Sold/Deleted), PhysicalCondition
├── dtos.rs                # CreateListingDto, UpdateListingDto, ListingResponseDto, PaginatedListingsDto
├── handlers/
│   ├── mod.rs
│   ├── list.rs            # GET /listings
│   ├── create.rs          # POST /listings
│   ├── get_by_id.rs       # GET /listings/:id
│   ├── update.rs          # PUT /listings/:id
│   ├── delete.rs          # DELETE /listings/:id
│   └── upload_image.rs    # POST /listings/:id/images
├── usecases/
│   ├── mod.rs
│   ├── create_listing_usecase.rs
│   ├── get_listing_usecase.rs
│   ├── update_listing_usecase.rs
│   ├── delete_listing_usecase.rs
│   └── upload_image_usecase.rs
└── adapters/
    ├── mod.rs
    ├── listing_repository.rs
    └── cloudinary.rs       # upload_image(), delete_image(), fallback /static/uploads/
```

**Traits expuestos**:
- `ListingRepository`: find_by_id, find_all_paginated, find_by_seller, insert, update, soft_delete
- `ImageStorage`: upload, delete, get_optimized_url

**Dependencias**: common, users

---

### 3.5 crate: search

**Responsabilidad**: Búsqueda full-text con MeiliSearch, indexación de anuncios y fallback SQL ILIKE.

```
crates/search/src/
├── lib.rs
├── router.rs
├── errors.rs
├── models.rs              # SearchResult, SearchFilters
├── dtos.rs                # SearchQueryDto { q, category, min_price, max_price, lat, lng, page }, SearchResponseDto
├── handlers/
│   ├── mod.rs
│   └── search.rs          # GET /search
├── usecases/
│   ├── mod.rs
│   ├── search_usecase.rs
│   └── index_listing_usecase.rs
└── adapters/
    ├── mod.rs
    ├── meilisearch_adapter.rs   # index_listing(), remove_listing(), search()
    └── sql_fallback.rs          # ILIKE fallback cuando MeiliSearch no disponible
```

**Traits expuestos**:
- `SearchEngine`: search, index_listing, remove_listing

**Dependencias**: common, listings

---

### 3.6 crate: chat

**Responsabilidad**: Mensajería en tiempo real WebSocket, gestión de conversaciones, persistencia de mensajes y fallback HTTP polling.

```
crates/chat/src/
├── lib.rs
├── router.rs
├── errors.rs              # ChatError: ConversationNotFound, NotMember, InvalidMessage
├── models.rs              # Conversation, Message
├── dtos.rs                # CreateConversationDto, SendMessageDto, MessageResponseDto, ConversationResponseDto
├── connections.rs         # ActiveConnections: DashMap<(Uuid,Uuid), UnboundedSender<WsMessage>>
├── handlers/
│   ├── mod.rs
│   ├── list_conversations.rs   # GET /chat
│   ├── create_conversation.rs  # POST /chat
│   ├── get_messages.rs         # GET /chat/:id/messages
│   ├── send_message.rs         # POST /chat/:id/messages
│   └── ws_handler.rs           # WS /chat/:id/ws
├── usecases/
│   ├── mod.rs
│   ├── list_conversations_usecase.rs
│   ├── create_conversation_usecase.rs
│   ├── get_messages_usecase.rs
│   ├── send_message_usecase.rs
│   ├── ws_lifecycle_usecase.rs      # handle_socket() con tokio::select!
│   └── process_message_usecase.rs   # Persistir + broadcast
└── adapters/
    ├── mod.rs
    ├── conversation_repository.rs
    └── message_repository.rs
```

**Traits expuestos**:
- `ConversationRepository`: find_by_id, find_by_user, create, update_last_message, is_member
- `MessageRepository`: create, find_by_conversation, count_unread

**Dependencias**: common, users, listings

---

### 3.7 crate: payments

**Responsabilidad**: Integración Stripe, creación de PaymentIntents, verificación de webhooks HMAC y gestión del estado de transacciones.

```
crates/payments/src/
├── lib.rs
├── router.rs
├── errors.rs              # PaymentError: StripeError, InvalidSignature, NotFound, Forbidden
├── models.rs              # Payment, PaymentStatus(Pending/Succeeded/Failed/Refunded)
├── dtos.rs                # CreateIntentDto, CreateIntentResponse, PaymentStatusDto
├── handlers/
│   ├── mod.rs
│   ├── create_intent.rs   # POST /payments/intent
│   ├── webhook.rs         # POST /payments/webhook (sin JWT, con firma Stripe)
│   └── get_status.rs      # GET /payments/:id/status
├── usecases/
│   ├── mod.rs
│   ├── create_intent_usecase.rs
│   ├── handle_webhook_usecase.rs
│   └── get_payment_status_usecase.rs
└── adapters/
    ├── mod.rs
    ├── stripe_adapter.rs       # create_payment_intent() con async-stripe
    └── payment_repository.rs   # insert, update_status, find_by_id
```

**Traits expuestos**:
- `PaymentRepository`: insert, update_status, find_by_id
- `StripePort`: create_payment_intent, verify_webhook_signature

**Dependencias**: common, users, listings

---

### 3.8 crate: ratings

**Responsabilidad**: Valoraciones post-transacción con validación de unicidad y Value Object RatingScore(1-5).

```
crates/ratings/src/
├── lib.rs
├── router.rs
├── errors.rs              # RatingError: AlreadyRated, InvalidScore, TransactionNotCompleted
├── models.rs              # Rating, RatingScore(Value Object 1-5)
├── dtos.rs                # CreateRatingDto, RatingDto, RatingsListDto
├── handlers/
│   ├── mod.rs
│   ├── create_rating.rs   # POST /listings/:id/ratings
│   └── list_ratings.rs    # GET /users/:id/ratings
├── usecases/
│   ├── mod.rs
│   ├── create_rating_usecase.rs
│   └── list_ratings_usecase.rs
└── adapters/
    ├── mod.rs
    └── rating_repository.rs   # insert (con check unicidad), find_by_listing, find_by_user
```

**Traits expuestos**:
- `RatingRepository`: insert, find_by_listing_id, find_by_user_id, calculate_average

**Dependencias**: common, users, listings

---

### 3.9 crate: favorites

**Responsabilidad**: Gestión de anuncios guardados con operaciones idempotentes.

```
crates/favorites/src/
├── lib.rs
├── router.rs
├── errors.rs              # FavoriteError: NotFound, AlreadyExists
├── models.rs              # Favorite
├── dtos.rs                # FavoriteDto, FavoritesListDto
├── handlers/
│   ├── mod.rs
│   ├── add_favorite.rs    # POST /listings/:id/favorites
│   ├── remove_favorite.rs # DELETE /listings/:id/favorites
│   └── list_favorites.rs  # GET /users/me/favorites
├── usecases/
│   ├── mod.rs
│   ├── add_favorite_usecase.rs
│   ├── remove_favorite_usecase.rs
│   └── list_favorites_usecase.rs
└── adapters/
    ├── mod.rs
    └── favorite_repository.rs  # insert (idempotente), delete, find_by_user
```

**Traits expuestos**:
- `FavoriteRepository`: insert, delete, find_by_user_id, exists

**Dependencias**: common, users, listings

---

### 3.10 crate: geo

**Responsabilidad**: Búsqueda de anuncios por proximidad geográfica usando PostGIS ST_DWithin.

```
crates/geo/src/
├── lib.rs
├── router.rs
├── errors.rs              # GeoError: InvalidCoordinates, RadiusExceeded
├── models.rs              # GeoPoint { lat: f64, lng: f64 }, GeoListing
├── dtos.rs                # GeoSearchQuery { lat, lng, radius_m, limit }, GeoListingDto { ...listing, distance_m }
├── handlers/
│   ├── mod.rs
│   └── search_by_geo.rs   # GET /listings/nearby
├── usecases/
│   ├── mod.rs
│   └── geo_search_usecase.rs
└── adapters/
    ├── mod.rs
    └── geo_repository.rs   # ST_DWithin(location, ST_MakePoint($lng,$lat)::geography, $radius)
```

**Traits expuestos**:
- `GeoRepository`: search_nearby(lat, lng, radius_m, limit)

**Dependencias**: common, listings

---

### 3.11 crate: api

**Responsabilidad**: Orquestador principal. Inicializa AppState, monta todos los routers, configura middleware y arranca el servidor Axum.

```
crates/api/src/
├── main.rs                # Punto de entrada: carga .env, init AppState, arranca servidor
├── app_state.rs           # AppState { pool, jwt_secret, active_connections, stripe_client, ... }
├── router.rs              # Monta todos los sub-routers bajo sus prefijos
├── errors.rs              # AppError global con IntoResponse
├── auth_extractor.rs      # AuthUser extractor (FromRequestParts) reutilizado por todos los crates
└── middleware/
    ├── mod.rs
    ├── cors.rs            # tower-http CorsLayer
    ├── tracing.rs         # tower-http TraceLayer
    └── rate_limit.rs      # Rate limiting en /auth/login y /auth/register
```

**AppState completo**:
```rust
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub active_connections: Arc<DashMap<(Uuid, Uuid), UnboundedSender<Message>>>,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub cloudinary_url: String,
    pub meili_client: Client,
}
```

**Dependencias**: todos los crates del workspace

---

## 4. Contratos de API (OpenAPI simplificado)

### 4.1 Módulo: auth

#### POST /auth/register
- **Auth**: Público
- **Request body**:
```json
{
  "email": "string (email válido)",
  "password": "string (mín 8 chars)",
  "displayName": "string (mín 2 chars)",
  "phone": "string (opcional)"
}
```
- **Response 201**:
```json
{
  "accessToken": "string (JWT)",
  "expiresIn": 86400,
  "user": { "id": "uuid", "email": "string", "displayName": "string" }
}
```
- **Errores**: 400 (validación), 409 (email ya existe), 500

#### POST /auth/login
- **Auth**: Público
- **Request body**:
```json
{ "email": "string", "password": "string" }
```
- **Response 200**:
```json
{ "accessToken": "string (JWT)", "expiresIn": 86400 }
```
- **Errores**: 400, 401 (credenciales incorrectas — mensaje genérico), 429 (rate limit)

#### POST /auth/refresh
- **Auth**: Bearer JWT (puede estar próximo a expirar)
- **Request body**: `{}`
- **Response 200**: `{ "accessToken": "string", "expiresIn": 86400 }`
- **Errores**: 401 (token inválido o expirado)

#### POST /auth/logout
- **Auth**: Bearer JWT
- **Response 200**: `{ "message": "Sesión cerrada" }`
- **Nota**: Stateless — el cliente descarta el token

---

### 4.2 Módulo: users

#### GET /users/:id
- **Auth**: Público
- **Path params**: `id: uuid`
- **Response 200**:
```json
{
  "id": "uuid",
  "displayName": "string",
  "avatarUrl": "string | null",
  "ratingAvg": "number",
  "totalRatings": "integer",
  "createdAt": "datetime"
}
```
- **Errores**: 404 (usuario no encontrado)
- **Nota**: NUNCA incluye email, passwordHash ni tokens

---

### 4.3 Módulo: listings

#### GET /listings
- **Auth**: Público
- **Query params**: `page=0`, `per_page=20`, `category=string`, `status=active`
- **Response 200**:
```json
{
  "data": [{ "id": "uuid", "title": "string", "price": "number", "currency": "eur", "imageUrl": "string|null", "city": "string", "condition": "new|like_new|used", "createdAt": "datetime" }],
  "page": 0, "perPage": 20, "total": 150
}
```
- **Errores**: 400 (params inválidos)

#### POST /listings
- **Auth**: Bearer JWT
- **Request body**:
```json
{
  "title": "string (mín 3, máx 100)",
  "description": "string (máx 2000)",
  "price": "number (> 0, máx 999999.99)",
  "category": "string",
  "condition": "new|like_new|used",
  "locationLat": "number",
  "locationLon": "number",
  "city": "string"
}
```
- **Response 201**: ListingResponseDto completo
- **Errores**: 400 (validación), 401

#### GET /listings/:id
- **Auth**: Público
- **Response 200**: ListingResponseDto con imágenes y datos del vendedor
- **Errores**: 404

#### PUT /listings/:id
- **Auth**: Bearer JWT (debe ser el vendedor)
- **Request body**: Mismos campos de POST pero todos opcionales (PATCH semantics)
- **Response 200**: ListingResponseDto actualizado
- **Errores**: 400, 401, 403 (no es el propietario), 404

#### DELETE /listings/:id
- **Auth**: Bearer JWT (debe ser el vendedor o admin)
- **Response 204**: Sin body
- **Nota**: Soft delete — status cambia a 'deleted'
- **Errores**: 401, 403, 404

#### POST /listings/:id/images
- **Auth**: Bearer JWT (debe ser el vendedor)
- **Request body**: multipart/form-data con campo `image` (JPEG/PNG/WebP, máx 5MB)
- **Response 201**: `{ "id": "uuid", "imageUrl": "string", "position": 0 }`
- **Errores**: 400 (tipo/tamaño inválido), 401, 403, 404, 422 (ya tiene 10 imágenes)

---

### 4.4 Módulo: search

#### GET /search
- **Auth**: Público
- **Query params**: `q=string`, `category=string`, `minPrice=number`, `maxPrice=number`, `lat=number`, `lng=number`, `page=0`, `perPage=20`
- **Response 200**:
```json
{
  "data": [ListingSummaryDto],
  "total": 42,
  "page": 0,
  "perPage": 20,
  "engine": "meilisearch|sql_fallback"
}
```
- **Errores**: 400 (params inválidos)

---

### 4.5 Módulo: chat

#### GET /chat
- **Auth**: Bearer JWT
- **Query params**: `page=0`, `perPage=20`
- **Response 200**:
```json
{
  "conversations": [{ "id": "uuid", "listingId": "uuid", "listingTitle": "string", "otherUserId": "uuid", "otherUserName": "string", "lastMessage": "string|null", "unreadCount": 0, "updatedAt": "datetime" }],
  "total": 5
}
```

#### POST /chat
- **Auth**: Bearer JWT
- **Request body**: `{ "listingId": "uuid", "initialMessage": "string (mín 1, máx 5000)" }`
- **Response 201**: ConversationResponseDto
- **Errores**: 400 (mensaje vacío), 404 (listing no encontrado), 409 (conversación ya existe), 422 (intentar chatear consigo mismo)

#### GET /chat/:id/messages
- **Auth**: Bearer JWT (debe ser miembro)
- **Query params**: `since=datetime (opcional)`, `limit=50`
- **Response 200**: `[MessageResponseDto]`
- **Nota**: Marca automáticamente mensajes del otro usuario como leídos
- **Errores**: 401, 403, 404

#### POST /chat/:id/messages
- **Auth**: Bearer JWT (debe ser miembro)
- **Request body**: `{ "content": "string (mín 1, máx 5000)" }`
- **Response 201**: MessageResponseDto
- **Errores**: 400, 401, 403, 404

#### WS /chat/:id/ws
- **Auth**: Query param `?token=<jwt>` (validado antes del upgrade)
- **Upgrade**: WebSocket
- **Mensajes entrantes**: `{ "content": "string" }`
- **Mensajes salientes**: MessageResponseDto serializado
- **Errores pre-upgrade**: 401 (token inválido), 403 (no es miembro)

---

### 4.6 Módulo: payments

#### POST /payments/intent
- **Auth**: Bearer JWT (comprador)
- **Request body**: `{ "listingId": "uuid", "currency": "eur" }`
- **Response 201**: `{ "paymentId": "uuid", "clientSecret": "string (Stripe)" }`
- **Errores**: 400, 401, 404 (listing no encontrado), 422 (intentar comprar propio anuncio)

#### POST /payments/webhook
- **Auth**: Firma Stripe-Signature (HMAC-SHA256) — SIN JWT
- **Request body**: Raw bytes (no parsear antes de verificar firma)
- **Response 200**: `{ "received": true }`
- **Errores**: 400 (firma inválida)

#### GET /payments/:id/status
- **Auth**: Bearer JWT (buyer o seller del pago)
- **Response 200**: `{ "id": "uuid", "status": "pending|succeeded|failed|refunded", "amountCents": 1500, "currency": "eur", "createdAt": "datetime" }`
- **Errores**: 401, 403, 404

---

### 4.7 Módulo: ratings

#### POST /listings/:id/ratings
- **Auth**: Bearer JWT
- **Request body**: `{ "score": 1-5, "comment": "string (opcional, máx 500)" }`
- **Response 201**: RatingDto
- **Errores**: 400, 401, 409 (ya valoró), 422 (score fuera de rango)

#### GET /users/:id/ratings
- **Auth**: Público
- **Query params**: `page=0`, `perPage=20`
- **Response 200**: `{ "data": [RatingDto], "total": 10, "averageScore": 4.5 }`
- **Errores**: 404

---

### 4.8 Módulo: favorites

#### POST /listings/:id/favorites
- **Auth**: Bearer JWT
- **Response 200**: `{ "added": true }` (idempotente — 200 aunque ya existía)
- **Errores**: 401, 404

#### DELETE /listings/:id/favorites
- **Auth**: Bearer JWT
- **Response 204**: Sin body
- **Errores**: 401, 404

#### GET /users/me/favorites
- **Auth**: Bearer JWT
- **Query params**: `page=0`, `perPage=20`
- **Response 200**: `{ "data": [ListingSummaryDto], "total": 8 }`
- **Errores**: 401

---

### 4.9 Módulo: geo

#### GET /listings/nearby
- **Auth**: Público
- **Query params**: `lat=number`, `lng=number`, `radius=number (metros, máx 50000)`, `limit=20 (máx 100)`
- **Response 200**: `{ "data": [{ ...ListingSummaryDto, "distanceM": 350.5 }], "total": 12 }`
- **Errores**: 400 (coords inválidas o radio > 50km)

---

## 5. Esquema de Base de Datos

### 5.1 Extensiones requeridas

```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS postgis;
```

### 5.2 Tablas

#### users
```sql
CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    display_name    TEXT NOT NULL CHECK (length(display_name) >= 2),
    avatar_url      TEXT,
    phone           TEXT,
    role            TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin')),
    rating_avg      NUMERIC(3,2) DEFAULT 0.00 CHECK (rating_avg >= 0 AND rating_avg <= 5),
    total_ratings   INTEGER NOT NULL DEFAULT 0,
    last_login_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_users_email ON users(email);
```

#### listings
```sql
CREATE TABLE listings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    seller_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           TEXT NOT NULL CHECK (length(title) >= 3 AND length(title) <= 100),
    description     TEXT CHECK (length(description) <= 2000),
    price           NUMERIC(10,2) NOT NULL CHECK (price > 0 AND price <= 999999.99),
    currency        TEXT NOT NULL DEFAULT 'eur',
    category        TEXT NOT NULL,
    condition       TEXT NOT NULL CHECK (condition IN ('new', 'like_new', 'used')),
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'sold', 'deleted')),
    location_lat    FLOAT8,
    location_lon    FLOAT8,
    location        GEOGRAPHY(POINT, 4326),
    city            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_listings_seller ON listings(seller_id);
CREATE INDEX idx_listings_status ON listings(status);
CREATE INDEX idx_listings_category ON listings(category);
CREATE INDEX idx_listings_price ON listings(price);
CREATE INDEX idx_listings_created ON listings(created_at DESC);
CREATE INDEX idx_listings_location ON listings USING GIST(location);
```

#### listing_images
```sql
CREATE TABLE listing_images (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id  UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    image_url   TEXT NOT NULL,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_listing_images_listing ON listing_images(listing_id);
```

#### conversations
```sql
CREATE TABLE conversations (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id          UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    buyer_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    seller_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_message        TEXT,
    last_message_at     TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(listing_id, buyer_id)
);

CREATE INDEX idx_conversations_buyer ON conversations(buyer_id);
CREATE INDEX idx_conversations_seller ON conversations(seller_id);
CREATE INDEX idx_conversations_updated ON conversations(updated_at DESC);
```

#### messages
```sql
CREATE TABLE messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content         TEXT NOT NULL CHECK (length(content) >= 1 AND length(content) <= 5000),
    is_read         BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_messages_conversation ON messages(conversation_id);
CREATE INDEX idx_messages_created ON messages(created_at ASC);
CREATE INDEX idx_messages_unread ON messages(conversation_id, is_read) WHERE is_read = false;
```

#### payments
```sql
CREATE TABLE payments (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id                  UUID NOT NULL REFERENCES listings(id),
    buyer_id                    UUID NOT NULL REFERENCES users(id),
    seller_id                   UUID NOT NULL REFERENCES users(id),
    stripe_payment_intent_id    TEXT NOT NULL UNIQUE,
    amount_cents                BIGINT NOT NULL CHECK (amount_cents > 0),
    currency                    TEXT NOT NULL DEFAULT 'eur',
    status                      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'succeeded', 'failed', 'refunded')),
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_payments_buyer ON payments(buyer_id);
CREATE INDEX idx_payments_seller ON payments(seller_id);
CREATE INDEX idx_payments_listing ON payments(listing_id);
CREATE INDEX idx_payments_stripe ON payments(stripe_payment_intent_id);
```

#### ratings
```sql
CREATE TABLE ratings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id      UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    rater_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rated_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    score           SMALLINT NOT NULL CHECK (score >= 1 AND score <= 5),
    comment         TEXT CHECK (length(comment) <= 500),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(listing_id, rater_id)
);

CREATE INDEX idx_ratings_rated ON ratings(rated_id);
CREATE INDEX idx_ratings_listing ON ratings(listing_id);
```

#### favorites
```sql
CREATE TABLE favorites (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    listing_id  UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, listing_id)
);

CREATE INDEX idx_favorites_user ON favorites(user_id);
```

### 5.3 Diagrama relacional

```
users
  ├── listings (seller_id)
  │     └── listing_images (listing_id)
  ├── conversations (buyer_id, seller_id)
  │     └── messages (conversation_id, sender_id)
  ├── payments (buyer_id, seller_id)
  ├── ratings (rater_id, rated_id)
  └── favorites (user_id)
              └── listings (listing_id)
```

---

## 6. Flujos de Procesos Complejos

### 6.1 Flujo de autenticación JWT

```mermaid
sequenceDiagram
    participant C as Cliente (Browser)
    participant A as Axum API
    participant DB as PostgreSQL
    participant J as JWT Module

    Note over C,J: Registro
    C->>A: POST /auth/register { email, password, displayName }
    A->>A: Validar DTO (email formato, password >= 8 chars)
    A->>DB: SELECT * FROM users WHERE email = $1
    DB-->>A: null (email libre)
    A->>J: hash_password(password) con Argon2id OWASP
    J-->>A: password_hash
    A->>DB: INSERT INTO users (email, password_hash, display_name)
    DB-->>A: User { id, email, ... }
    A->>J: generate_jwt(user_id, role, exp=now+24h)
    J-->>A: access_token (JWT HS256)
    A-->>C: 201 { accessToken, expiresIn: 86400, user }

    Note over C,J: Login
    C->>A: POST /auth/login { email, password }
    A->>DB: SELECT * FROM users WHERE email = $1
    DB-->>A: User | null
    A->>J: verify_password(password, user.password_hash)
    J-->>A: true | false
    alt Credenciales correctas
        A->>J: generate_jwt(user_id, role, exp)
        J-->>A: access_token
        A->>DB: UPDATE users SET last_login_at = now()
        A-->>C: 200 { accessToken, expiresIn }
    else Credenciales incorrectas
        A-->>C: 401 { error: "Credenciales incorrectas" }
    end

    Note over C,J: Request autenticado
    C->>A: GET /listings (Bearer: access_token)
    A->>J: verify_jwt(token, jwt_secret)
    J-->>A: Claims { sub, role, exp } | Error
    alt Token válido
        A->>DB: SELECT * FROM listings WHERE status = 'active'
        DB-->>A: listings[]
        A-->>C: 200 { data, total, page }
    else Token expirado
        A-->>C: 401 { error: "El token de sesión ha expirado" }
    end
```

### 6.2 Flujo de pago con Stripe

```mermaid
sequenceDiagram
    participant B as Comprador (Browser)
    participant A as Axum API
    participant DB as PostgreSQL
    participant S as Stripe API

    B->>A: POST /payments/intent { listingId, currency: "eur" }
    A->>A: Verificar AuthUser (JWT válido)
    A->>DB: SELECT * FROM listings WHERE id = $1 AND status = 'active'
    DB-->>A: Listing { price, seller_id }
    A->>A: Verificar buyer_id != seller_id
    A->>S: CreatePaymentIntent { amount_cents, currency, metadata: { listing_id, buyer_id } }
    S-->>A: PaymentIntent { id, client_secret, status: "requires_payment_method" }
    A->>DB: INSERT INTO payments (listing_id, buyer_id, seller_id, stripe_payment_intent_id, amount_cents, status: "pending")
    DB-->>A: Payment { id }
    A-->>B: 201 { paymentId, clientSecret }

    Note over B,S: El comprador introduce datos de tarjeta en Stripe Checkout
    B->>S: Confirmar pago (Stripe.js con clientSecret)
    S-->>B: Redirige a success_url o cancel_url

    Note over S,DB: Stripe notifica resultado via webhook
    S->>A: POST /payments/webhook (body raw, header Stripe-Signature)
    A->>A: Leer body como Bytes (ANTES de parsear)
    A->>A: HMAC-SHA256 verify(body, Stripe-Signature, webhook_secret)
    alt Firma válida
        A->>A: Parsear evento JSON
        alt payment_intent.succeeded
            A->>DB: UPDATE payments SET status='succeeded' WHERE stripe_payment_intent_id=$1
            A->>DB: UPDATE listings SET status='sold' WHERE id=$2
            A-->>S: 200 { received: true }
        else payment_intent.payment_failed
            A->>DB: UPDATE payments SET status='failed' WHERE stripe_payment_intent_id=$1
            A-->>S: 200 { received: true }
        end
    else Firma inválida
        A-->>S: 400 { error: "Invalid signature" }
    end
```

### 6.3 Flujo de búsqueda con MeiliSearch

```mermaid
sequenceDiagram
    participant C as Cliente
    participant A as Axum API
    participant M as MeiliSearch
    participant DB as PostgreSQL

    C->>A: GET /search?q=bicicleta&category=sport&maxPrice=200
    A->>A: Validar y parsear SearchQueryDto
    A->>M: search("bicicleta", filters: "category=sport AND price <= 200", limit: 20)
    alt MeiliSearch disponible
        M-->>A: SearchResult { hits: [listing_ids], totalHits: 42 }
        A->>DB: SELECT * FROM listings WHERE id = ANY($1) AND status = 'active'
        DB-->>A: listings[]
        A-->>C: 200 { data, total: 42, engine: "meilisearch" }
    else MeiliSearch no disponible (timeout/error)
        A->>DB: SELECT * FROM listings WHERE (title ILIKE '%bicicleta%' OR description ILIKE '%bicicleta%') AND category = 'sport' AND price <= 200 AND status = 'active' LIMIT 20
        DB-->>A: listings[]
        A-->>C: 200 { data, total, engine: "sql_fallback" }
    end

    Note over A,M: Indexación al crear/editar anuncio
    A->>M: index_listing({ id, title, description, price, category, city, status })
    M-->>A: Task { uid, status: "enqueued" }
```

### 6.4 Flujo de chat WebSocket

```mermaid
sequenceDiagram
    participant C1 as Comprador (Browser)
    participant C2 as Vendedor (Browser)
    participant A as Axum API
    participant DB as PostgreSQL
    participant AC as ActiveConnections (DashMap)

    Note over C1,AC: Establecer conexión WebSocket
    C1->>A: GET /chat/:id/ws?token=<jwt> (Upgrade: websocket)
    A->>A: verify_jwt(token) → Claims { sub: buyer_id }
    A->>DB: SELECT is_member(conversation_id, buyer_id)
    DB-->>A: true
    A->>AC: insert((conversation_id, buyer_id), tx_sender)
    A-->>C1: 101 Switching Protocols (WebSocket establecido)

    Note over C1,AC: Enviar mensaje en tiempo real
    C1->>A: WS Message: { "content": "¿Sigue disponible?" }
    A->>A: Deserializar SendMessageDto
    A->>A: Validar content (1-5000 chars)
    A->>DB: INSERT INTO messages (conversation_id, sender_id, content)
    DB-->>A: Message { id, created_at }
    A->>DB: UPDATE conversations SET last_message = $1, updated_at = now()
    A->>AC: get((conversation_id, seller_id))
    alt Vendedor conectado
        AC-->>A: Some(tx_sender)
        A->>C2: WS Message: MessageResponseDto (tiempo real)
    else Vendedor desconectado
        AC-->>A: None
        Note over A: Mensaje guardado en BD para polling posterior
    end

    Note over C1,AC: Desconexión y limpieza
    C1->>A: Cierre de conexión WebSocket
    A->>AC: remove((conversation_id, buyer_id))
```

---

## 7. Estrategia de Despliegue

### 7.1 Dockerfile multistage

```dockerfile
# ── Etapa 1: Builder ──────────────────────────────────────────────
FROM rust:1.78-slim AS builder

WORKDIR /app

# Instalar dependencias del sistema
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copiar manifiestos primero (aprovechar cache de Docker)
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build en modo release
RUN cargo build --release --bin api

# ── Etapa 2: Runtime ─────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Instalar solo dependencias de runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copiar binario compilado
COPY --from=builder /app/target/release/api ./nebripop

# Copiar migraciones y templates
COPY migrations/ ./migrations/
COPY templates/ ./templates/
COPY static/ ./static/

# Puerto de exposición
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["./nebripop"]
```

### 7.2 docker-compose.yml (entorno local)

```yaml
version: "3.9"

services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://nebripop:nebripop@postgres:5432/nebripop
      JWT_SECRET: ${JWT_SECRET}
      STRIPE_SECRET_KEY: ${STRIPE_SECRET_KEY}
      STRIPE_WEBHOOK_SECRET: ${STRIPE_WEBHOOK_SECRET}
      CLOUDINARY_URL: ${CLOUDINARY_URL}
      MEILISEARCH_URL: http://meilisearch:7700
      MEILISEARCH_MASTER_KEY: ${MEILISEARCH_MASTER_KEY}
      RUST_LOG: info
    depends_on:
      postgres:
        condition: service_healthy
      meilisearch:
        condition: service_healthy

  postgres:
    image: postgis/postgis:15-3.4
    environment:
      POSTGRES_USER: nebripop
      POSTGRES_PASSWORD: nebripop
      POSTGRES_DB: nebripop
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U nebripop"]
      interval: 10s
      timeout: 5s
      retries: 5

  meilisearch:
    image: getmeili/meilisearch:v1.45.0
    environment:
      MEILI_MASTER_KEY: ${MEILISEARCH_MASTER_KEY}
      MEILI_ENV: development
    ports:
      - "7700:7700"
    volumes:
      - meilisearch_data:/meili_data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:7700/health"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  postgres_data:
  meilisearch_data:
```

### 7.3 Variables de entorno

| Variable | Descripción | Ejemplo | Obligatoria |
|----------|-------------|---------|-------------|
| `DATABASE_URL` | URL de conexión PostgreSQL | `postgres://user:pass@localhost:5432/nebripop` | ✅ Sí |
| `JWT_SECRET` | Secret para firmar JWT (mín 32 chars) | `super_secret_key_min_32_chars_here` | ✅ Sí |
| `STRIPE_SECRET_KEY` | Clave secreta Stripe (test o producción) | `sk_test_51...` | ✅ Sí |
| `STRIPE_WEBHOOK_SECRET` | Secret para verificar webhooks Stripe | `whsec_...` | ✅ Sí |
| `CLOUDINARY_URL` | URL de conexión Cloudinary con credenciales | `cloudinary://api_key:api_secret@cloud_name` | ⚠️ Opcional (fallback local) |
| `MEILISEARCH_URL` | URL del servidor MeiliSearch | `http://localhost:7700` | ⚠️ Opcional (fallback SQL) |
| `MEILISEARCH_MASTER_KEY` | Master key de MeiliSearch | `nebripop_master_key` | ⚠️ Opcional |
| `RUST_LOG` | Nivel de logging (error/warn/info/debug) | `info` | No |
| `PORT` | Puerto del servidor (default 8080) | `8080` | No |

### 7.4 Topología en Railway

```
Railway Project: nebripop
├── Service: nebripop-api
│     Image: Docker (desde GitHub repo)
│     Port: 8080
│     Start command: ./nebripop
│     Variables: todas las de la tabla 7.3
│
├── Service: nebripop-postgres
│     Template: PostgreSQL 15
│     Variable generada: DATABASE_URL
│
└── Service: nebripop-meilisearch
      Template: MeiliSearch
      Variable generada: MEILISEARCH_URL
```

### 7.5 Health checks

| Servicio | Endpoint | Respuesta esperada |
|----------|----------|-------------------|
| API Axum | `GET /health` | `200 { "status": "ok", "version": "1.0.0" }` |
| PostgreSQL | `pg_isready -U nebripop` | Exit code 0 |
| MeiliSearch | `GET http://localhost:7700/health` | `200 { "status": "available" }` |
