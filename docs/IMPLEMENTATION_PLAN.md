# Plan de Implementación — Nebripop

**Versión:** v1.0  
**Fecha:** 27 de mayo de 2026  
**Autor:** architect-agent  
**Horizonte:** 5 sprints de 2 días (10 días laborables, 1 semana natural efectiva de entrega)  
**Fuente de verdad:** `project-context.md`, `docs/PRD.md`, `docs/architecture.md`

---

## Índice

1. [Visión general del plan](#1-visión-general-del-plan)
2. [Equipo y disponibilidad](#2-equipo-y-disponibilidad)
3. [Sprint 0 — Setup del entorno y migraciones](#3-sprint-0--setup-del-entorno-y-migraciones)
4. [Sprint 1 — Auth + Core modules](#4-sprint-1--auth--core-modules)
5. [Sprint 2 — Listings + Search + UI/UX](#5-sprint-2--listings--search--uiux)
6. [Sprint 3 — Chat + Payments](#6-sprint-3--chat--payments)
7. [Sprint 4 — Testing + DevOps + Despliegue](#7-sprint-4--testing--devops--despliegue)
8. [Dependencias críticas entre tareas](#8-dependencias-críticas-entre-tareas)
9. [Riesgos por sprint y mitigación](#9-riesgos-por-sprint-y-mitigación)
10. [Criterios de done globales](#10-criterios-de-done-globales)

---

## 1. Visión general del plan

```mermaid
gantt
    title Nebripop — Roadmap de sprints
    dateFormat  YYYY-MM-DD
    section Sprint 0 (Día 1-2)
    Setup entorno y servicios     :s0a, 2026-05-27, 1d
    Migraciones Base de Datos     :s0b, 2026-05-27, 2d
    section Sprint 1 (Día 3-4)
    crate users + Auth JWT        :s1a, 2026-05-29, 2d
    crate common + AppState       :s1b, 2026-05-29, 1d
    crate ratings + favorites     :s1c, 2026-05-29, 2d
    section Sprint 2 (Día 5-6)
    crate listings + Cloudinary   :s2a, 2026-05-31, 2d
    crate search + MeiliSearch    :s2b, 2026-05-31, 2d
    crate geo (Haversine)         :s2c, 2026-05-31, 1d
    Templates Askama UI/UX        :s2d, 2026-05-31, 2d
    section Sprint 3 (Día 7-8)
    crate chat + WebSockets       :s3a, 2026-06-02, 2d
    crate payments + Stripe       :s3b, 2026-06-02, 2d
    section Sprint 4 (Día 9-10)
    Tests integración             :s4a, 2026-06-04, 1d
    Dockerfile + CI/CD            :s4b, 2026-06-04, 1d
    Despliegue Railway            :s4c, 2026-06-05, 1d
    Demo E2E                      :s4d, 2026-06-05, 1d
```

### Leyenda de agentes responsables

| Agente | Herramienta | Modelo | Rol |
|--------|------------|--------|-----|
| `architect-agent` | Antigravity | claude-opus-4-6-thinking | Arquitectura y documentación |
| `reviewer-agent` | Antigravity | claude-sonnet-4-6-thinking | Gate de calidad (activo en todos los sprints) |
| `security-agent` | Antigravity | claude-sonnet-4-6-thinking | Auditoría de seguridad |
| `uiux-agent` | Antigravity | gemini-3.5-flash-high | Templates Askama + TailwindCSS |
| `devops-agent` | Antigravity | gemini-3.5-flash-medium | Dockerfile, CI/CD, Railway |
| `docs-agent` | Antigravity | gemini-3.5-flash-medium | Documentación y ai_log |
| `db-schema-agent` | OpenCode | qwen2.5-coder:7b | Migraciones SQL y esquema |
| `auth-agent` | OpenCode | qwen2.5-coder:7b | crate users + JWT |
| `codegen-listings-agent` | OpenCode | qwen2.5-coder:7b | crate listings + Cloudinary |
| `codegen-search-agent` | OpenCode | qwen2.5-coder:7b | crate search + MeiliSearch |
| `codegen-chat-agent` | OpenCode | qwen2.5-coder:7b | crate chat + WebSockets |
| `codegen-payments-agent` | OpenCode | qwen2.5-coder:7b | crate payments + Stripe |
| `codegen-core-agent` | OpenCode | qwen2.5-coder:7b | crates ratings, favorites, geo |
| `qa-agent` | OpenCode | qwen2.5-coder:7b | Tests de integración |

---

## 2. Equipo y disponibilidad

| Miembro | Disponibilidad | Perfil | Módulos asignados |
|---------|---------------|--------|-------------------|
| **Persona A** (lead técnico) | Alta — 6-8h/día | Rust, arquitectura | `users`, `listings`, `api` (AppState + routing) |
| **Persona B** (backend) | Media — 4-5h/día | Backend, APIs externas | `chat`, `payments`, `geo` |
| **Persona C** (fullstack/ops) | Baja — 2-3h/día | Frontend, DevOps | `search`, UI/UX templates, `devops` (Docker, CI/CD) |

### Distribución detallada por sprint

```
Sprint 0:
  Persona A → Cargo workspace, crate common, AppState skeleton
  Persona B → Cuentas servicios externos (Stripe, Cloudinary, MeiliSearch)
  Persona C → Docker Compose local, variables .env, setup GitHub repo

Sprint 1:
  Persona A → crate users (register, login, JWT middleware, AuthUser extractor)
  Persona B → crate ratings, crate favorites (dominio + adapters)
  Persona C → crate geo (Haversine SQL), documentación ai_log

Sprint 2:
  Persona A → crate listings (CRUD + Cloudinary adapter)
  Persona B → crate search (MeiliSearch + fallback SQL)
  Persona C → Templates Askama (base layout, listings feed, search, detalle)

Sprint 3:
  Persona A → crate api (montar todos los routers, revisar integración)
  Persona B → crate chat (WebSockets, DashMap, fallback polling) + crate payments (Stripe)
  Persona C → Templates Askama (chat UI, checkout Stripe, perfil usuario)

Sprint 4:
  Persona A → Tests integración crates users + listings + payments
  Persona B → Tests integración chat + search
  Persona C → Dockerfile multistage, GitHub Actions CI, despliegue Railway
```

> **Nota sobre sincronización:** Al final de cada sprint se hace un merge synchronization meeting (15 min asíncrono en texto via GitHub Issue). El PR gates de `reviewer-agent` se activan automáticamente en cada push a `main`.

---

## 3. Sprint 0 — Setup del entorno y migraciones

**Duración:** 2 días (Día 1-2)  
**Objetivo:** Infraestructura lista para que el equipo pueda codificar sin bloqueos de configuración.

---

### TAREA S0-01 — Inicializar Cargo workspace

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `db-schema-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.1 (Cargo.toml workspace) |
| **Output esperado** | `Cargo.toml` raíz con todos los members, `crates/common/`, `crates/api/src/main.rs` (stub) |
| **Precondiciones** | Repositorio Git inicializado en GitHub |
| **Definición de Done** | `cargo check` pasa sin errores; los 10 crates están declarados en `[workspace.members]`; `cargo tree` muestra dependencias compartidas en `[workspace.dependencies]` |

**Subtareas:**
1. Crear `Cargo.toml` raíz con `resolver = "2"` y todos los `[workspace.dependencies]`
2. Crear esqueleto de directorio `crates/{api,common,users,listings,search,chat,payments,ratings,favorites,geo}/src/lib.rs`
3. Verificar que `cargo build` compila sin errores de dependencias

---

### TAREA S0-02 — Docker Compose para desarrollo local

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `devops-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | `project-context.md`, `docs/architecture.md` §7 (variables de entorno) |
| **Output esperado** | `docker-compose.yml`, `.env.example`, `scripts/setup_services.sh` |
| **Precondiciones** | Docker Desktop instalado en la máquina del equipo |
| **Definición de Done** | `docker-compose up -d` levanta PostgreSQL 15 y MeiliSearch 1.x; `scripts/setup_services.sh` valida conectividad con todos los servicios y devuelve exit 0 |

**Servicios en docker-compose.yml:**
```yaml
# PostgreSQL 15 en puerto 5432
# MeiliSearch 1.x en puerto 7700
# Volume persistente para datos
```

---

### TAREA S0-03 — Migraciones de base de datos (8 tablas)

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `db-schema-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/PRD.md` §6 (modelo de datos), `docs/architecture.md` §5 (esquema físico) |
| **Output esperado** | `migrations/001_create_users.sql`, `migrations/002_create_listings.sql`, `migrations/003_create_listing_images.sql`, `migrations/004_create_conversations.sql`, `migrations/005_create_messages.sql`, `migrations/006_create_transactions.sql`, `migrations/007_create_ratings.sql`, `migrations/008_create_favorites.sql` |
| **Precondiciones** | S0-02 completada (PostgreSQL accesible en localhost:5432) |
| **Definición de Done** | `sqlx migrate run` aplica las 8 migraciones sin error; `postgres-mcp` confirma que las 8 tablas existen con sus constraints, índices y tipos correctos (UUID, TIMESTAMPTZ, NUMERIC) |

**MCP activo:** `postgres-mcp` — verificar esquema tras cada migración

---

### TAREA S0-04 — Variables de entorno y gestión de secrets

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `devops-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | `docs/architecture.md` §7, `project-context.md` (MCPs activos) |
| **Output esperado** | `.env.example` completo, `.gitignore` actualizado (excluir `.env`), `docs/setup-guide.md` |
| **Precondiciones** | Cuentas creadas en Stripe, Cloudinary, MeiliSearch por Persona B |
| **Definición de Done** | `.env.example` contiene todas las variables (`DATABASE_URL`, `JWT_SECRET`, `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `CLOUDINARY_URL`, `MEILI_URL`, `MEILI_MASTER_KEY`); `.env` está en `.gitignore`; `docs/setup-guide.md` explica cómo poplar el `.env` |

**Variables de entorno requeridas:**
```
DATABASE_URL=postgres://nebripop:nebripop@localhost:5432/nebripop_db
JWT_SECRET=<mínimo 32 chars aleatorios>
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...
CLOUDINARY_URL=cloudinary://api_key:api_secret@cloud_name
MEILI_URL=http://localhost:7700
MEILI_MASTER_KEY=<master key>
RUST_LOG=info
HOST=0.0.0.0
PORT=8080
```

---

### TAREA S0-05 — Documentación inicial ai_log (Sprint 0)

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `docs-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | `project-context.md`, outputs de S0-01 a S0-04 |
| **Output esperado** | `docs/ai_log/sprint-0.md` con decisiones, prompts usados y herramientas invocadas |
| **Precondiciones** | Ninguna |
| **Definición de Done** | `docs/ai_log/sprint-0.md` existe y documenta cada decisión tomada en Sprint 0 con su justificación |

---

### Checklist de cierre Sprint 0

- [ ] `cargo check` sin errores
- [ ] Docker Compose levanta PostgreSQL + MeiliSearch
- [ ] 8 migraciones aplicadas y verificadas con `postgres-mcp`
- [ ] `.env.example` completo
- [ ] Todas las cuentas externas creadas y API keys obtenidas
- [ ] `docs/ai_log/sprint-0.md` escrito

---

## 4. Sprint 1 — Auth + Core modules

**Duración:** 2 días (Día 3-4)  
**Objetivo:** El sistema puede registrar y autenticar usuarios; los módulos de soporte (ratings, favorites, geo) tienen su dominio implementado.

---

### TAREA S1-01 — crate `common`: tipos compartidos y errores

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `auth-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.2 (crate common), skill `error-handling-rust`, skill `rust-domain-modeling` |
| **Output esperado** | `crates/common/src/{lib.rs, errors.rs, pagination.rs, ids.rs, response.rs}` |
| **Precondiciones** | S0-01 completada (Cargo workspace) |
| **Definición de Done** | `AppError` implementa `IntoResponse` para Axum; los newtypes `UserId`, `ListingId`, `ChatId`, `PaymentId` implementan `Serialize`/`Deserialize`; `cargo test -p common` pasa |

**Skills invocadas:** `error-handling-rust`, `rust-domain-modeling`

---

### TAREA S1-02 — crate `users`: registro, login y JWT middleware

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `auth-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.3 (crate users), `docs/PRD.md` §5 (US-01, US-02, US-03), skill `jwt-auth-rust`, skill `axum-best-practices` |
| **Output esperado** | `crates/users/src/{lib.rs, router.rs, models.rs, dtos.rs, handlers/, usecases/, adapters/}` |
| **Precondiciones** | S1-01 completada, S0-03 completada (tabla `users` en BD) |
| **Definición de Done** | `POST /auth/register` crea usuario con Argon2id y devuelve JWT 24h; `POST /auth/login` devuelve JWT o HTTP 401; `AuthUser` extractor falla con HTTP 401 si token inválido; `cargo test -p users` pasa (mínimo 3 tests) |

**MCPs activos:** `postgres-mcp` (verificar inserciones), `github-mcp` (PR review)  
**Skills invocadas:** `jwt-auth-rust`, `axum-best-practices`, `error-handling-rust`, `clean-code-rust`, `solid-rust`

**Gate de revisión:** `reviewer-agent` revisa el PR antes de merge a `main`. Checklist obligatorio:
- No hay `unwrap()` en código de producción
- Argon2id con parámetros OWASP (m=19456, t=2, p=1)
- JWT_SECRET se lee de variable de entorno, no hardcodeado

---

### TAREA S1-03 — crate `api`: AppState y skeleton del servidor

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `auth-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.11 (crate api + AppState), skill `axum-best-practices` |
| **Output esperado** | `crates/api/src/{main.rs, app_state.rs, router.rs, auth_extractor.rs, middleware/}` |
| **Precondiciones** | S1-02 completada |
| **Definición de Done** | `cargo run -p api` arranca en puerto 8080; `GET /health` devuelve HTTP 200; middleware de CORS y tracing activos; pool SQLx conectado a PostgreSQL |

**Skills invocadas:** `axum-best-practices`

---

### TAREA S1-04 — crate `ratings`: valoraciones post-transacción

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `codegen-core-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.8 (crate ratings), `docs/PRD.md` §5 (US-12, US-13), skill `sqlx-best-practices`, skill `rust-domain-modeling` |
| **Output esperado** | `crates/ratings/src/{lib.rs, router.rs, models.rs, dtos.rs, handlers/, usecases/, adapters/}` |
| **Precondiciones** | S1-01, S0-03 (tabla `ratings`) |
| **Definición de Done** | `POST /listings/:id/ratings` crea valoración (1-5) con constraint de unicidad por transacción; `GET /users/:id/ratings` devuelve promedio + últimas 10; `cargo test -p ratings` pasa |

**Skills invocadas:** `sqlx-best-practices`, `rust-domain-modeling`, `error-handling-rust`

---

### TAREA S1-05 — crate `favorites`: gestión de favoritos

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `codegen-core-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.9 (crate favorites), `docs/PRD.md` §5 (US-16), skill `sqlx-best-practices` |
| **Output esperado** | `crates/favorites/src/{lib.rs, router.rs, models.rs, dtos.rs, handlers/, usecases/, adapters/}` |
| **Precondiciones** | S1-01, S0-03 (tabla `favorites`) |
| **Definición de Done** | `POST /listings/:id/favorites` es idempotente (no error si ya existe); `DELETE /listings/:id/favorites` elimina; cascade delete activo (verificar con `postgres-mcp`); toggle funciona en 2 llamadas consecutivas |

**MCPs activos:** `postgres-mcp` (verificar CASCADE)

---

### TAREA S1-06 — crate `geo`: búsqueda por proximidad

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `codegen-core-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.10 (crate geo), `docs/PRD.md` §5 (US-14, US-15), skill `sqlx-best-practices` |
| **Output esperado** | `crates/geo/src/{lib.rs, router.rs, models.rs, dtos.rs, handlers/, usecases/, adapters/geo_repository.rs}` |
| **Precondiciones** | S1-01, S0-03 (columnas lat/lon en `listings`) |
| **Definición de Done** | `GET /listings/nearby?lat=X&lng=Y&radius_m=10000` devuelve listings ordenados por distancia; query SQL usa fórmula Haversine nativa (sin extensión PostGIS adicional); filtros 5/10/25/50 km funcionan |

---

### TAREA S1-07 — Auditoría de seguridad Sprint 1

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `security-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | PRs de S1-02, S1-03; `docs/architecture.md` §ADR-004; skill `security-audit-rust`, skill `jwt-auth-rust` |
| **Output esperado** | `docs/ai_log/security-sprint1.md` con hallazgos y estado (resuelto/mitigado) |
| **Precondiciones** | S1-02, S1-03 mergeados |
| **Definición de Done** | Checklist de seguridad completo: JWT_SECRET en env, no `unwrap()`, Argon2id parámetros OWASP, rate limiting en `/auth/login`, no SQL injection en queries |

---

### Checklist de cierre Sprint 1

- [ ] `POST /auth/register` y `POST /auth/login` funcionan end-to-end
- [ ] `AuthUser` extractor bloquea requests sin JWT válido (HTTP 401)
- [ ] crates ratings, favorites, geo compilan y pasan tests básicos
- [ ] `cargo build` sin warnings en modo release
- [ ] Auditoría de seguridad Sprint 1 completa
- [ ] `docs/ai_log/sprint-1.md` escrito

---

## 5. Sprint 2 — Listings + Search + UI/UX

**Duración:** 2 días (Día 5-6)  
**Objetivo:** El marketplace tiene anuncios funcionales, búsqueda operativa y frontend visible.

---

### TAREA S2-01 — crate `listings`: CRUD completo + Cloudinary

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `codegen-listings-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.4 (crate listings), `docs/PRD.md` §5 (US-04, US-05, US-06), skill `cloudinary-integration`, skill `rust-axum-handler`, skill `sqlx-best-practices` |
| **Output esperado** | `crates/listings/src/{lib.rs, router.rs, models.rs, dtos.rs, handlers/{list,create,get_by_id,update,delete,upload_image}.rs, usecases/, adapters/{listing_repository,cloudinary}.rs}` |
| **Precondiciones** | S1-01, S1-02 (AuthUser extractor listo), S0-03 (tablas listings + listing_images) |
| **Definición de Done** | 6 endpoints funcionan según PRD §4.3; imágenes se suben a Cloudinary y URL se persiste en `listing_images`; solo el propietario puede editar/eliminar (HTTP 403 en caso contrario); soft delete cambia `status` a `deleted`; `cargo test -p listings` con mínimo 2 tests |

**MCPs activos:** `postgres-mcp` (verificar queries), `github-mcp` (PR)  
**Skills invocadas:** `rust-axum-handler`, `sqlx-best-practices`, `rust-domain-modeling`, `cloudinary-integration`, `clean-code-rust`, `error-handling-rust`

**Gate de revisión:** `reviewer-agent` debe aprobar antes de merge. Verificar:
- Authorization check en handlers de update/delete
- Validación de tipo MIME antes de subir a Cloudinary
- Fallback a `/static/uploads/` si Cloudinary falla

---

### TAREA S2-02 — crate `search`: MeiliSearch + fallback SQL

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `codegen-search-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.5 (crate search), `docs/PRD.md` §5 (US-07, US-08), skill `meilisearch-integration`, skill `rust-axum-handler` |
| **Output esperado** | `crates/search/src/{lib.rs, router.rs, models.rs, dtos.rs, handlers/search.rs, usecases/, adapters/{meilisearch_adapter,sql_fallback}.rs}` |
| **Precondiciones** | S2-01 completada (listings deben existir para indexar), S0-02 (MeiliSearch accesible) |
| **Definición de Done** | `GET /search?q=texto` devuelve resultados en <300ms usando MeiliSearch; filtros por categoría, minPrice, maxPrice funcionan combinables; si MeiliSearch no disponible, el sistema cae en fallback SQL ILIKE sin error 500; campo `engine` en la respuesta indica cuál se usó |

**MCPs activos:** `meilisearch-mcp` (crear índice `listings_index`, configurar atributos filtrables), `postgres-mcp`  
**Skills invocadas:** `meilisearch-integration`, `rust-axum-handler`, `sqlx-best-practices`, `error-handling-rust`, `clean-code-rust`

**Configuración MeiliSearch requerida:**
```
Índice: listings_index
searchableAttributes: [title, description, city]
filterableAttributes: [category, price, status, location_lat, location_lon]
sortableAttributes: [price, created_at]
typoTolerance: enabled
```

---

### TAREA S2-03 — Templates Askama: layout base + páginas principales

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `uiux-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | `docs/PRD.md` §3 (actores), `docs/architecture.md` §ADR-002 (Askama + Tailwind CDN), skill `askama-best-practices`, skill `askama-template`, skill `tailwind-patterns` |
| **Output esperado** | `crates/api/templates/{base.html, index.html, listings/list.html, listings/detail.html, search/results.html, auth/login.html, auth/register.html, users/profile.html}` |
| **Precondiciones** | S2-01 completada (modelos ListingResponseDto disponibles para templates) |
| **Definición de Done** | Todas las páginas renderizan sin error de compilación Askama; diseño responsive funciona en 375px y 1920px; TailwindCSS CDN cargado; navegación funcional entre pages; `cargo build -p api` sin errores de templates |

**Skills invocadas:** `askama-best-practices`, `askama-template`, `tailwind-patterns`

**Páginas obligatorias:**
- `base.html` — layout con navbar, footer, CDN TailwindCSS
- `index.html` — feed principal de anuncios con búsqueda rápida
- `listings/list.html` — grid de anuncios con pagination
- `listings/detail.html` — detalle anuncio + botón favorito + botón contactar
- `listings/create.html` — formulario crear anuncio con upload de imagen
- `search/results.html` — resultados de búsqueda con filtros laterales
- `auth/login.html` y `auth/register.html` — formularios de autenticación
- `users/profile.html` — perfil público con valoraciones y listings

---

### TAREA S2-04 — Integración listings → MeiliSearch (indexación automática)

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `codegen-listings-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | S2-01 (crate listings), S2-02 (crate search), `docs/architecture.md` §ADR-005 |
| **Output esperado** | Actualización de `create_listing_usecase.rs` y `update_listing_usecase.rs` para indexar en MeiliSearch tras persistir en PostgreSQL |
| **Precondiciones** | S2-01 y S2-02 completadas |
| **Definición de Done** | Al crear un anuncio, aparece indexado en MeiliSearch en menos de 5 segundos; al eliminar/marcar vendido, se elimina del índice; consistencia eventual verificada con `meilisearch-mcp` |

**MCPs activos:** `meilisearch-mcp`

---

### TAREA S2-05 — Montar routers listings + search + geo + ratings + favorites en crate `api`

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `codegen-core-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.11 (crate api router), outputs S2-01 a S2-02, S1-04 a S1-06 |
| **Output esperado** | `crates/api/src/router.rs` actualizado con todos los sub-routers bajo sus prefijos |
| **Precondiciones** | S2-01, S2-02, S1-04, S1-05, S1-06 completadas y compilando |
| **Definición de Done** | `cargo run -p api` sirve todos los endpoints de listings, search, geo, ratings y favorites; `GET /health` sigue funcionando; middleware de auth activo en rutas protegidas |

---

### Checklist de cierre Sprint 2

- [ ] `cargo build` sin errores con todos los crates de Sprint 2
- [ ] CRUD de listings funciona end-to-end (crear → ver → editar → borrar)
- [ ] Imagen se sube a Cloudinary y URL se guarda en BD
- [ ] Búsqueda por texto libre y filtros funcionan (<300ms)
- [ ] Fallback SQL activo cuando MeiliSearch no disponible
- [ ] 8 páginas Askama renderizan sin error en móvil y desktop
- [ ] `docs/ai_log/sprint-2.md` escrito

---

## 6. Sprint 3 — Chat + Payments

**Duración:** 2 días (Día 7-8)  
**Objetivo:** El flujo completo de compraventa funciona: chatear + pagar + valorar.

---

### TAREA S3-01 — crate `chat`: WebSockets + persistencia + fallback polling

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `codegen-chat-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.6 (crate chat) + §ADR-008 (WebSockets), `docs/PRD.md` §5 (US-09, US-10, US-11), skill `websocket-rust`, skill `rust-axum-handler` |
| **Output esperado** | `crates/chat/src/{lib.rs, router.rs, models.rs, dtos.rs, connections.rs, handlers/{list_conversations,create_conversation,get_messages,send_message,ws_handler}.rs, usecases/, adapters/}` |
| **Precondiciones** | S1-02 (AuthUser extractor), S2-01 (crate listings — conversaciones vinculadas a listings), S0-03 (tablas conversations + messages) |
| **Definición de Done** | WebSocket `WS /chat/:id/ws` acepta conexiones con JWT por query param; mensajes llegan en <100ms; mensajes se persisten en PostgreSQL; `GET /chat/:id/messages?since=X` funciona como fallback HTTP; indicador de mensajes no leídos (`unreadCount`) correcto; `cargo test -p chat` pasa |

**MCPs activos:** `postgres-mcp` (verificar conversaciones y mensajes), `github-mcp` (PR)  
**Skills invocadas:** `websocket-rust`, `rust-axum-handler`, `sqlx-best-practices`, `error-handling-rust`, `clean-code-rust`

**Gate de revisión:** `reviewer-agent` verifica:
- DashMap usado correctamente (no Mutex global)
- `tokio::select!` en ws_lifecycle_usecase
- No hay panic posible en el loop del WebSocket

---

### TAREA S3-02 — crate `payments`: Stripe PaymentIntent + webhook

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `codegen-payments-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/architecture.md` §3.7 (crate payments) + §ADR-006 (Stripe), `docs/PRD.md` §5 (US-17, US-18), skill `stripe-integration`, skill `security-audit-rust` |
| **Output esperado** | `crates/payments/src/{lib.rs, router.rs, models.rs, dtos.rs, handlers/{create_intent,webhook,get_status}.rs, usecases/, adapters/{stripe_adapter,payment_repository}.rs}` |
| **Precondiciones** | S1-02 (AuthUser), S2-01 (listings — necesario para marking sold), S0-03 (tabla transactions), API key Stripe obtenida en S0-04 |
| **Definición de Done** | `POST /payments/intent` crea PaymentIntent en Stripe modo test y devuelve `client_secret`; `POST /payments/webhook` valida firma HMAC `Stripe-Signature` y actualiza transacción a `paid` + listing a `sold`; cero datos de tarjeta en BD; `cargo test -p payments` pasa (mínimo 2 tests) |

**MCPs activos:** `stripe-mcp` (verificar PaymentIntents en dashboard test), `postgres-mcp`, `github-mcp`  
**Skills invocadas:** `stripe-integration`, `rust-axum-handler`, `sqlx-best-practices`, `error-handling-rust`, `clean-code-rust`, `security-audit-rust`

**Gate de seguridad:** `security-agent` audita el handler del webhook antes de merge:
- Validación de `Stripe-Signature` es obligatoria
- Header raw body leído ANTES del deserialización JSON
- Webhook endpoint excluido del middleware JWT (es llamado por Stripe, no por usuarios)

---

### TAREA S3-03 — Templates Askama: chat UI + checkout Stripe

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `uiux-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | S3-01 (dtos de chat), S3-02 (dtos de payments), skill `askama-template`, skill `tailwind-patterns` |
| **Output esperado** | `crates/api/templates/{chat/conversation.html, chat/list.html, payments/checkout.html, payments/success.html, payments/error.html}` |
| **Precondiciones** | S3-01 y S3-02 completadas (DTOs disponibles para templates) |
| **Definición de Done** | Chat UI muestra lista de conversaciones y ventana de mensajes en tiempo real via WebSocket JS vanilla; botón "Pagar" redirige a Stripe Checkout; páginas de éxito y error post-pago renderizan correctamente |

**Skills invocadas:** `askama-template`, `tailwind-patterns`

---

### TAREA S3-04 — Integración completa en crate `api`: montar chat + payments

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `codegen-core-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | S3-01, S3-02; `docs/architecture.md` §3.11 |
| **Output esperado** | `crates/api/src/router.rs` con rutas de chat y payments montadas |
| **Precondiciones** | S3-01 y S3-02 compilando |
| **Definición de Done** | `cargo run -p api` sirve TODOS los endpoints del PRD §4; flujo completo registro → listing → chat → pago funciona end-to-end en local |

---

### TAREA S3-05 — Auditoría de seguridad Sprint 3

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `security-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | PRs S3-01, S3-02; skill `security-audit-rust` |
| **Output esperado** | `docs/ai_log/security-sprint3.md` |
| **Precondiciones** | S3-01, S3-02 mergeados |
| **Definición de Done** | Audit completo de webhook Stripe (firma HMAC), WebSocket JWT auth, SQL injection en queries de chat, no datos de tarjeta en logs |

---

### Checklist de cierre Sprint 3

- [ ] WebSocket chat funciona en tiempo real (<100ms latencia)
- [ ] Mensajes persisten en PostgreSQL
- [ ] Fallback HTTP polling activo
- [ ] PaymentIntent creado en Stripe test mode
- [ ] Webhook actualiza transacción + listing correctamente
- [ ] Firma Stripe-Signature validada (auditoría superada)
- [ ] Flujo E2E completo funciona en local
- [ ] `docs/ai_log/sprint-3.md` escrito

---

## 7. Sprint 4 — Testing + DevOps + Despliegue

**Duración:** 2 días (Día 9-10)  
**Objetivo:** Sistema production-ready con tests, CI/CD y URL pública.

---

### TAREA S4-01 — Tests de integración: auth + listings + search

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `qa-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | `docs/PRD.md` §10 (criterios de éxito), crates users, listings, search; skill `rust-integration-test` |
| **Output esperado** | `crates/users/tests/integration_auth.rs`, `crates/listings/tests/integration_listings.rs`, `crates/search/tests/integration_search.rs` |
| **Precondiciones** | Sprint 1 y 2 completados. PostgreSQL accesible para `sqlx::test` |
| **Definición de Done** | Mínimo 8 tests de integración (happy path de cada Must Have): registro, login, token inválido, crear listing, editar listing (propio), editar listing (403), búsqueda básica, favorito toggle |

**Skills invocadas:** `rust-integration-test`, `error-handling-rust`

---

### TAREA S4-02 — Tests de integración: chat + payments + ratings

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `qa-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | Crates chat, payments, ratings; skill `rust-integration-test` |
| **Output esperado** | `crates/chat/tests/integration_chat.rs`, `crates/payments/tests/integration_payments.rs`, `crates/ratings/tests/integration_ratings.rs` |
| **Precondiciones** | Sprint 3 completado |
| **Definición de Done** | Mínimo 6 tests: crear conversación, enviar mensaje, crear PaymentIntent, webhook mock, crear rating, duplicar rating (error) |

---

### TAREA S4-03 — Dockerfile multistage optimizado

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `devops-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | `docs/architecture.md` §7 (estrategia de despliegue), skill `docker-rust` |
| **Output esperado** | `Dockerfile` multistage (imagen final < 50MB), `.dockerignore` |
| **Precondiciones** | `cargo build --release` funciona |
| **Definición de Done** | `docker build -t nebripop .` completa sin errores; `docker run --env-file .env nebripop` arranca y expone puerto 8080; imagen final < 50MB verificado con `docker image ls` |

**Skills invocadas:** `docker-rust`

**Estructura multistage:**
```dockerfile
# Stage 1: builder (imagen rust con cargo-chef)
# Stage 2: runtime (debian-slim o distroless, solo el binario)
```

---

### TAREA S4-04 — GitHub Actions CI/CD pipeline

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `devops-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | S4-03 (Dockerfile), S4-01/S4-02 (tests), skill `github-actions-rust`, `github-mcp` |
| **Output esperado** | `.github/workflows/ci.yml`, `.github/workflows/deploy.yml` |
| **Precondiciones** | S4-01, S4-02, S4-03 completadas |
| **Definición de Done** | `ci.yml` ejecuta `cargo check`, `cargo test` y `cargo clippy -- -D warnings` en cada PR; `deploy.yml` se activa en push a `main` y despliega en Railway automáticamente |

**Skills invocadas:** `github-actions-rust`  
**MCPs activos:** `github-mcp` (crear workflows via API), `railway-mcp` (configurar deploy automático)

---

### TAREA S4-05 — Despliegue en Railway

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `devops-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | S4-03 (Dockerfile), S4-04 (CI/CD), `docs/architecture.md` §7 (variables de entorno) |
| **Output esperado** | URL pública de Railway activa, `docs/deployment.md` con instrucciones de despliegue |
| **Precondiciones** | S4-03, cuenta Railway configurada |
| **Definición de Done** | URL pública responde HTTP 200 en `GET /health`; PostgreSQL y MeiliSearch accesibles desde Railway; variables de entorno configuradas en Railway dashboard; URL documentada en README.md |

**MCPs activos:** `railway-mcp`

---

### TAREA S4-06 — Tests E2E con Playwright: flujos críticos

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `qa-agent` (OpenCode) |
| **Herramienta** | OpenCode |
| **Input necesario** | URL pública de S4-05, `docs/PRD.md` §10 (criterios de éxito), `playwright-mcp` |
| **Output esperado** | `tests/e2e/{register_flow.spec.ts, listing_create.spec.ts, search_flow.spec.ts, payment_flow.spec.ts}` |
| **Precondiciones** | S4-05 completada (app desplegada) |
| **Definición de Done** | 4 tests E2E pasan contra la URL de producción: (1) registro → login → logout; (2) crear anuncio con imagen; (3) buscar anuncio; (4) flujo de pago Stripe test mode completo |

**MCPs activos:** `playwright-mcp`

---

### TAREA S4-07 — Documentación final y README

| Campo | Detalle |
|-------|---------|
| **Agente responsable** | `docs-agent` (Antigravity) |
| **Herramienta** | Antigravity |
| **Input necesario** | Todos los `docs/ai_log/sprint-*.md`, outputs de todos los sprints |
| **Output esperado** | `README.md` actualizado con URL de producción, instrucciones de setup local, `docs/ai_log/sprint-4.md` |
| **Precondiciones** | S4-05 completada |
| **Definición de Done** | README contiene: descripción del proyecto, URL pública, instrucciones `docker-compose up`, lista de endpoints, stack tecnológico; todos los `ai_log` de sprints 0-4 existen |

---

### Checklist de cierre Sprint 4 (= criterios de entrega)

- [ ] Mínimo 14 tests de integración pasan (`cargo test`)
- [ ] `cargo clippy -- -D warnings` sin errores
- [ ] Dockerfile genera imagen < 50MB
- [ ] GitHub Actions CI pasa en verde
- [ ] URL pública Railway responde correctamente
- [ ] 4 tests E2E Playwright pasan
- [ ] README actualizado con URL y setup guide
- [ ] Todos los 12 criterios mínimos del PRD §10 cumplidos

---

## 8. Dependencias críticas entre tareas

```mermaid
graph TD
    S0-01[S0-01 Cargo workspace] --> S1-01[S1-01 crate common]
    S0-02[S0-02 Docker Compose] --> S0-03[S0-03 Migraciones]
    S0-03 --> S1-02[S1-02 crate users]
    S1-01 --> S1-02
    S1-02 --> S1-03[S1-03 crate api AppState]

    S1-01 --> S1-04[S1-04 crate ratings]
    S1-01 --> S1-05[S1-05 crate favorites]
    S1-01 --> S1-06[S1-06 crate geo]
    S0-03 --> S1-04
    S0-03 --> S1-05
    S0-03 --> S1-06

    S1-02 --> S2-01[S2-01 crate listings]
    S0-03 --> S2-01
    S2-01 --> S2-02[S2-02 crate search]
    S2-01 --> S2-03[S2-03 Templates Askama]

    S2-01 --> S2-04[S2-04 Indexación MeiliSearch]
    S2-02 --> S2-04

    S1-04 --> S2-05[S2-05 Montar routers Sprint 2]
    S1-05 --> S2-05
    S1-06 --> S2-05
    S2-01 --> S2-05
    S2-02 --> S2-05

    S2-01 --> S3-01[S3-01 crate chat]
    S1-02 --> S3-01
    S0-03 --> S3-01

    S2-01 --> S3-02[S3-02 crate payments]
    S1-02 --> S3-02
    S0-03 --> S3-02
    S0-04[S0-04 Variables .env] --> S3-02

    S3-01 --> S3-03[S3-03 Templates chat]
    S3-02 --> S3-03

    S3-01 --> S3-04[S3-04 Integración final]
    S3-02 --> S3-04

    S3-04 --> S4-01[S4-01 Tests auth+listings]
    S3-04 --> S4-02[S4-02 Tests chat+payments]
    S3-04 --> S4-03[S4-03 Dockerfile]

    S4-03 --> S4-04[S4-04 GitHub Actions]
    S4-01 --> S4-04
    S4-02 --> S4-04

    S4-04 --> S4-05[S4-05 Railway deploy]
    S4-05 --> S4-06[S4-06 Tests E2E]
    S4-05 --> S4-07[S4-07 README final]
```

### Bloqueos críticos identificados

| Tarea bloqueante | Tareas bloqueadas | Impacto si falla |
|-----------------|-------------------|-----------------|
| **S0-03** (Migraciones) | S1-02, S1-04, S1-05, S1-06, S2-01, S3-01, S3-02 | **CRÍTICO** — Bloquea todo Sprint 1 y posteriores |
| **S1-02** (crate users + AuthUser) | S2-01, S3-01, S3-02 | **CRÍTICO** — Sin auth no hay endpoints protegidos |
| **S2-01** (crate listings) | S2-02, S2-03, S2-04, S3-01, S3-02 | **ALTO** — Bloquea search, chat y payments |
| **S0-04** (API keys Stripe/Cloudinary/MeiliSearch) | S2-01, S2-02, S3-02 | **ALTO** — Sin API keys no se pueden inicializar adapters |
| **S3-04** (integración final routers) | S4-01, S4-02, S4-06 | **MEDIO** — Bloquea testing y deploy |

---

## 9. Riesgos por sprint y mitigación

### Sprint 0 — Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| Cuentas externas (Stripe, Cloudinary, MeiliSearch) tardan en activarse | Media | Alto | Crear cuentas en el Día 0 antes de arrancar Sprint 0; Persona B es responsable exclusivo |
| `sqlx migrate run` falla por tipo de datos incompatible | Baja | Medio | Usar `postgres-mcp` para verificar cada migración individualmente; revertir con `sqlx migrate revert` |
| Conflictos de versiones en `Cargo.toml` workspace | Media | Bajo | Seguir exactamente las versiones de `docs/architecture.md` §3.1; no usar wildcards `*` |

### Sprint 1 — Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| `opencode` con `qwen2.5-coder:7b` genera código Rust con unwrap() o sin error handling | Alta | Alto | `reviewer-agent` como gate obligatorio en cada PR; checklist de no-unwrap explícito |
| Argon2id más lento de lo esperado en máquinas de desarrollo | Baja | Bajo | Usar parámetros mínimos OWASP (m=19456, no incrementar); descartar benchmarks en dev |
| JWT_SECRET hardcodeado en código por error del agente | Media | Crítico | `security-agent` audita Sprint 1 antes de merge a main |

### Sprint 2 — Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| Cloudinary cambia su API o el SDK falla | Baja | Alto | Implementar fallback `/static/uploads/` desde el inicio (parte del ADR-007) |
| MeiliSearch indexación inconsistente (listing creado pero no indexado) | Media | Medio | Indexar sincrónicamente como primera versión; consistencia eventual como optimización posterior |
| Templates Askama complejas no compilan por errores de tipos | Media | Medio | `uiux-agent` usa tipos de DTOs del PRD como referencia; compilar incrementalmente template a template |
| Retraso de Persona C (baja disponibilidad) bloquea UI | Alta | Medio | Los templates pueden hacerse más simples en Sprint 2 y pulirse en Sprint 4; funcionalidad primero |

### Sprint 3 — Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| WebSockets en Rust difíciles de debuggear | Alta | Alto | Implementar primero HTTP polling como fallback funcional; WS como mejora; `codegen-chat-agent` usa skill `websocket-rust` con patrones validados |
| Webhook de Stripe no llega en local (requiere túnel) | Media | Alto | Usar `stripe listen --forward-to localhost:8080/payments/webhook` en desarrollo; en producción se resuelve con Railway URL pública |
| Validación de firma Stripe incorrecta | Media | Crítico | `security-agent` audita S3-02 obligatoriamente antes de merge |
| crate `payments` referencia `stripe-rust` API incorrecta | Media | Alto | `codegen-payments-agent` usa skill `stripe-integration` con ejemplos validados; testar con `stripe-mcp` en modo test |

### Sprint 4 — Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| Tests de integración con `sqlx::test` lentos en CI | Baja | Bajo | Usar `--test-threads=4` y BD efímera paralela |
| Railway no despliega correctamente (variables de entorno mal configuradas) | Media | Alto | Script de validación `scripts/validate_env.sh` que corre antes del deploy |
| Playwright tests flaky por timing de WebSocket | Media | Medio | Usar `waitForEvent` de Playwright con timeout 5s; sleep explícito para operaciones async |
| Imagen Docker > 50MB | Baja | Bajo | `devops-agent` usa multistage con `cargo-chef` y base `debian-slim`; layer caché optimizado |

---

## 10. Criterios de done globales

### Criterios mínimos de entrega (PRD §10)

Todos estos criterios deben cumplirse en la URL de producción:

| # | Criterio | Sprint |
|---|---------|--------|
| 1 | Usuario puede registrarse y recibir JWT válido | Sprint 1 |
| 2 | JWT permite acceder a endpoints protegidos | Sprint 1 |
| 3 | Vendedor crea anuncio con ≥1 imagen en listado público | Sprint 2 |
| 4 | Vendedor puede editar y eliminar sus propios anuncios | Sprint 2 |
| 5 | Comprador puede buscar por texto y filtrar por categoría/precio | Sprint 2 |
| 6 | Dos usuarios pueden intercambiar mensajes via chat | Sprint 3 |
| 7 | Comprador completa pago Stripe test mode | Sprint 3 |
| 8 | Usuario valora a otro tras transacción completada | Sprint 1+3 |
| 9 | Anuncios muestran ciudad y filtran por distancia | Sprint 1 |
| 10 | Usuario puede marcar/desmarcar favoritos | Sprint 1 |
| 11 | `cargo build` sin errores, `cargo run` ejecuta | Sprint 0 |
| 12 | Documentación: PRD, arquitectura, ai_log completos | Todos |

### Criterios de excelencia (PRD §10)

| # | Criterio | Sprint | Probabilidad |
|---|---------|--------|-------------|
| 1 | Chat WebSocket real (no solo polling) | Sprint 3 | Alta |
| 2 | ≥1 test integración por módulo Must Have | Sprint 4 | Alta |
| 3 | Aplicación desplegada con URL pública | Sprint 4 | Alta |
| 4 | Diseño responsive en viewport 375px | Sprint 2 | Media |
| 5 | ≥2 funcionalidades Should Have completas | Sprint 2-3 | Media |
| 6 | Demo E2E en <3 minutos | Sprint 4 | Alta |
| 7 | Todos los commits generados por agentes IA | Todos | Alta |

---

## Apéndice A — Convención de commits

Todos los commits deben seguir el formato:

```
[agente] tipo(módulo): descripción breve

- Detalle adicional si necesario
- MCP usado: postgres-mcp / stripe-mcp / etc.
```

**Ejemplos:**
```
[auth-agent] feat(users): implement register endpoint with argon2id
[codegen-listings-agent] feat(listings): add Cloudinary upload adapter with fallback
[reviewer-agent] fix(users): remove unwrap() from JWT validation
[devops-agent] ci(github-actions): add cargo clippy check on PR
```

---

## Apéndice B — Convención de ramas

```
main          — rama principal protegida; merge solo via PR aprobado
feature/s0-*  — features de Sprint 0
feature/s1-*  — features de Sprint 1
feature/s2-*  — features de Sprint 2
feature/s3-*  — features de Sprint 3
feature/s4-*  — features de Sprint 4
hotfix/*      — correcciones urgentes de producción
```

**Regla:** Ningún agente hace push directo a `main`. Todo merge requiere PR revisado por `reviewer-agent`.

---

*Documento generado por `architect-agent` basado en `project-context.md`, `docs/PRD.md` y `docs/architecture.md`.*
