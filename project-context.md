# Nebripop — Contexto del proyecto

## Identificación
- **Nombre:** Nebripop
- **Descripción:** Clon funcional de Wallapop desarrollado íntegramente con IA
- **Repositorio:** https://github.com/victorcastillojimenez/Nebripop
- **Asignatura:** Desarrollo con IA — Nebrija
- **Entrega:** 1 semana desde inicio de desarrollo

## Equipo
- 3 personas con disponibilidad desigual
- Código manual = cero, todo generado por agentes IA
- Herramientas principales: opencode (bajo nivel) + Antigravity (alto nivel)

## Stack técnico
- **Backend:** Rust, Axum, SQLx, Tokio
- **Frontend:** Leptos (WASM)
- **Base de datos:** PostgreSQL
- **Autenticación:** JWT con jsonwebtoken + argon2
- **Búsqueda:** MeiliSearch
- **Pagos:** Stripe
- **Imágenes:** Cloudinary
- **Tiempo real:** WebSockets con tokio-tungstenite

## Metodología
- XP adaptado a desarrollo con IA
- Sprints de 2 días
- Cada tarea: prompt → generación → revisión → commit
- Todo prompt y decisión queda documentado en docs/ai_log/

## Arquitectura objetivo
- Arquitectura hexagonal por crates
- Un crate por dominio: users, listings, search, chat, payments
- Crate api como orquestador (Axum)
- Migraciones en /migrations con SQLx

## MCPs activos en el proyecto
- github-mcp
- postgres-mcp
- redis-mcp
- stripe-mcp (fase 4)
- cloudinary-mcp (fase 4)
- meilisearch-mcp (fase 4)
- figma-mcp (fase 6)
- playwright-mcp (fase 5)

## Scope resumido
### Must Have
- Registro y login de usuarios (JWT)
- CRUD de anuncios con imágenes
- Búsqueda y filtros básicos
- Mensajería entre usuarios
- Valoraciones post-transacción
- Geolocalización de anuncios
- Favoritos
- Pagos con Stripe

### Out of Scope
- Logística real con couriers
- Búsqueda por imagen con ML
- Panel de analítica avanzada
- Sistema de soporte/helpdesk
- Aplicación móvil nativa