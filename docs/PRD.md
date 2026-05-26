# PRD — Nebripop

**Versión:** v1.0
**Fecha:** 22 de mayo de 2026
**Equipo:** Nebripop
**Repositorio:** https://github.com/victorcastillojimenez/Nebripop

## 1. Resumen ejecutivo

Nebripop es un marketplace C2C de compraventa de artículos de segunda mano, clon funcional de Wallapop, desarrollado íntegramente mediante agentes de IA sin código manual. Dirigido a usuarios que quieren publicar, descubrir y comprar productos cercanos de forma segura. Resuelve la necesidad de conectar compradores y vendedores locales con chat integrado, pagos seguros y reputación verificable. La entrega comprometida incluye: autenticación JWT, CRUD de anuncios con imágenes, búsqueda con filtros, mensajería en tiempo real, valoraciones post-transacción, geolocalización, favoritos y pagos con Stripe. Stack: Rust (Axum + SQLx + Tokio) con templates Askama y PostgreSQL. Equipo de 3 personas, 1 semana de desarrollo real, sprints de 2 días.

---

## 2. Objetivos del proyecto

### Objetivos de negocio

1. Demostrar un flujo completo de compraventa C2C end-to-end (publicar → buscar → contactar → pagar → valorar)
2. Ofrecer una experiencia de usuario comparable a Wallapop en las funcionalidades core
3. Implementar pagos reales con Stripe que permitan transacciones seguras entre usuarios

### Objetivos técnicos

1. Construir un backend en Rust con arquitectura hexagonal por crates que compile sin errores y pase validación SQLx
2. Demostrar que agentes de IA pueden generar un sistema completo y funcional sin intervención manual de código
3. Alcanzar tiempos de respuesta de API < 200ms en el percentil 95

### Objetivos académicos

1. Evidenciar el proceso completo de ingeniería de software con IA: research → PRD → arquitectura → implementación → testing
2. Documentar cada decisión de diseño y prompt en `docs/ai_log/` como trazabilidad del desarrollo
3. Entregar un producto funcional desplegable que cumpla los requisitos del enunciado de la asignatura

---

## 3. Actores del sistema

| Actor | Descripción | Permisos principales |
|-------|-------------|---------------------|
| **Usuario anónimo** | Visitante no autenticado | Ver anuncios públicos, buscar y filtrar, ver perfiles públicos, registrarse |
| **Usuario registrado (comprador)** | Usuario autenticado que busca productos | Todo lo del anónimo + enviar mensajes, hacer ofertas, pagar con Stripe, valorar vendedores, marcar favoritos |
| **Usuario registrado (vendedor)** | Usuario autenticado que publica productos | Todo lo del comprador + crear/editar/eliminar anuncios propios, subir imágenes, recibir pagos, responder mensajes |
| **Administrador** | Operador del sistema con acceso privilegiado | Eliminar anuncios que violen normas, bloquear usuarios, acceder al panel de administración, gestionar reportes |

> **Nota:** Todo usuario registrado es simultáneamente comprador y vendedor. La distinción existe solo a nivel de permisos contextuales (sobre sus propios anuncios vs. los de otros).

### Detalle de acceso por endpoint

| Endpoint / Funcionalidad | Anónimo | Comprador | Vendedor | Admin |
|--------------------------|:-------:|:---------:|:--------:|:-----:|
| `GET /listings` — Ver anuncios | ✅ | ✅ | ✅ | ✅ |
| `GET /listings/:id` — Detalle anuncio | ✅ | ✅ | ✅ | ✅ |
| `GET /users/:id` — Perfil público | ✅ | ✅ | ✅ | ✅ |
| `POST /auth/register` — Registro | ✅ | — | — | — |
| `POST /auth/login` — Login | ✅ | ✅ | ✅ | ✅ |
| `GET /search` — Búsqueda con filtros | ✅ | ✅ | ✅ | ✅ |
| `POST /listings` — Crear anuncio | ❌ | ✅ | ✅ | ✅ |
| `PUT /listings/:id` — Editar anuncio propio | ❌ | ❌ | ✅ (propio) | ✅ |
| `DELETE /listings/:id` — Eliminar anuncio | ❌ | ❌ | ✅ (propio) | ✅ (cualquiera) |
| `POST /chat` — Enviar mensaje | ❌ | ✅ | ✅ | ✅ |
| `GET /chat` — Ver conversaciones | ❌ | ✅ | ✅ | ✅ |
| `POST /payments` — Iniciar pago Stripe | ❌ | ✅ | ❌ | ❌ |
| `POST /ratings` — Valorar usuario | ❌ | ✅ | ✅ | ✅ |
| `POST /favorites` — Guardar favorito | ❌ | ✅ | ✅ | ✅ |
| `PUT /users/me` — Editar perfil propio | ❌ | ✅ | ✅ | ✅ |
| `DELETE /users/:id` — Bloquear usuario | ❌ | ❌ | ❌ | ✅ |
| `GET /admin` — Panel administración | ❌ | ❌ | ❌ | ✅ |

---

## 4. Alcance del proyecto

### Must Have (8 funcionalidades)

| Funcionalidad | Prioridad | Justificación | Módulo Rust |
|---------------|-----------|---------------|-------------|
| Registro y login con JWT | Must Have | Sin autenticación no hay identidad ni permisos; bloquea todo lo demás | `users` |
| CRUD de anuncios con imágenes | Must Have | Core del marketplace; sin anuncios no existe el producto | `listings` |
| Búsqueda con filtros básicos | Must Have | El descubrimiento es la función principal del comprador; sin búsqueda no hay conversión | `search` |
| Mensajería en tiempo real | Must Have | Canal de negociación imprescindible; Wallapop sin chat no funciona | `chat` |
| Valoraciones post-transacción | Must Have | Genera confianza en un entorno C2C entre desconocidos; sin reputación no hay repetición | `ratings` |
| Geolocalización de anuncios | Must Have | La proximidad es el diferenciador clave frente a otros marketplaces generalistas | `geo` |
| Favoritos | Must Have | Retención directa: permite al comprador volver a productos de interés sin buscar de nuevo | `favorites` |
| Pagos con Stripe | Must Have | Cierra el flujo completo de transacción; sin pago seguro no hay venta verificable | `payments` |

### Should Have (5 funcionalidades)

| Funcionalidad | Prioridad | Justificación | Módulo Rust |
|---------------|-----------|---------------|-------------|
| Notificaciones in-app | Should Have | Mejora engagement pero el chat ya notifica vía WebSocket | `notifications` |
| Seguir a otros usuarios | Should Have | Incrementa retención a largo plazo pero no bloquea el flujo core de compraventa | `favorites` |
| Destacar anuncios (boost de pago) | Should Have | Monetización secundaria interesante pero no crítica para un MVP funcional | `listings` |
| Panel de administración básico | Should Have | Útil para moderación pero durante el MVP se puede operar directamente vía SQL | `admin` |
| Perfil público con estadísticas | Should Have | Mejora la confianza del comprador pero las valoraciones ya cubren lo mínimo necesario | `users` |

### Could Have (4 funcionalidades)

| Funcionalidad | Prioridad | Justificación | Módulo Rust |
|---------------|-----------|---------------|-------------|
| Búsquedas guardadas con alertas | Could Have | Nice-to-have de retención; no impacta el flujo principal | `search` |
| Modo oscuro | Could Have | Mejora la percepción de calidad UX pero no es funcional | `api` (frontend) |
| Compartir anuncios por enlace | Could Have | Viralidad orgánica con implementación trivial; depende de tener URLs públicas | `listings` |
| Historial de transacciones | Could Have | Útil para auditoría del usuario pero la información ya existe en la base de datos | `payments` |

### Out of Scope

| Funcionalidad excluida | Justificación |
|------------------------|---------------|
| Logística con couriers reales (Correos, SEUR) | Requiere contratos e integraciones con APIs de mensajería que exceden el tiempo y la complejidad operacional disponibles |
| Búsqueda por imagen con ML | Necesita infraestructura de visión computacional (modelos, GPU, indexación visual) inviable en 1 semana |
| Panel de analítica avanzada para vendedores | Las métricas detalladas (gráficas de visitas, tendencias, ingresos por periodo) son un nice-to-have que no impacta el flujo core |
| Sistema de soporte / helpdesk | Requiere un sistema de ticketing completo (tipo Zendesk); las incidencias se resuelven fuera de la plataforma en esta versión |
| Aplicación móvil nativa (iOS/Android) | El scope es exclusivamente web; el diseño responsive con TailwindCSS cubre la experiencia en dispositivos móviles |

---

## 5. User stories por módulo

### Módulo `users`

> **US-01** Como usuario anónimo quiero registrarme con email y contraseña para poder publicar y comprar productos
> - **Criterio de aceptación:** El sistema crea la cuenta, hashea la contraseña con argon2, devuelve un JWT válido y redirige al feed principal
> - **Módulo Rust:** `users`
> - **Prioridad:** Must Have

> **US-02** Como usuario registrado quiero iniciar sesión con mis credenciales para acceder a mi cuenta desde cualquier navegador
> - **Criterio de aceptación:** Con email y contraseña correctos se devuelve un JWT con expiración de 24h; con credenciales incorrectas se devuelve HTTP 401
> - **Módulo Rust:** `users`
> - **Prioridad:** Must Have

> **US-03** Como usuario registrado quiero editar mi perfil (nombre, foto, ubicación) para que otros usuarios me identifiquen
> - **Criterio de aceptación:** Los campos se actualizan en BD y el perfil público refleja los cambios en la siguiente carga de página
> - **Módulo Rust:** `users`
> - **Prioridad:** Must Have

### Módulo `listings`

> **US-04** Como vendedor quiero crear un anuncio con título, descripción, precio, categoría y fotos para ofrecer mi producto
> - **Criterio de aceptación:** El anuncio se persiste en PostgreSQL, las imágenes se suben a Cloudinary, y el anuncio aparece en la búsqueda en menos de 5 segundos
> - **Módulo Rust:** `listings`
> - **Prioridad:** Must Have

> **US-05** Como vendedor quiero editar o eliminar mis anuncios para mantener mi catálogo actualizado
> - **Criterio de aceptación:** Solo el propietario (`seller_id` = `user_id` del JWT) puede editar o eliminar; intentos de otros usuarios devuelven HTTP 403
> - **Módulo Rust:** `listings`
> - **Prioridad:** Must Have

> **US-06** Como vendedor quiero marcar un anuncio como vendido para que deje de aparecer en búsquedas activas
> - **Criterio de aceptación:** El campo `status` cambia a `sold`; el anuncio desaparece de resultados de búsqueda pero sigue visible en el perfil del vendedor como histórico
> - **Módulo Rust:** `listings`
> - **Prioridad:** Must Have

### Módulo `search`

> **US-07** Como comprador quiero buscar anuncios por texto libre para encontrar productos que me interesen
> - **Criterio de aceptación:** La búsqueda devuelve resultados relevantes en menos de 300ms usando MeiliSearch; si no hay resultados se muestra un mensaje de estado vacío
> - **Módulo Rust:** `search`
> - **Prioridad:** Must Have

> **US-08** Como comprador quiero filtrar resultados por categoría, rango de precio y distancia para refinar mi búsqueda
> - **Criterio de aceptación:** Cada filtro reduce los resultados correctamente; los filtros son combinables entre sí sin conflictos
> - **Módulo Rust:** `search`
> - **Prioridad:** Must Have

### Módulo `chat`

> **US-09** Como comprador quiero enviar un mensaje al vendedor desde un anuncio para negociar la compra
> - **Criterio de aceptación:** Se crea una conversación vinculada al listing; el mensaje llega al vendedor en tiempo real vía WebSocket; si el socket no está activo, el mensaje se persiste y aparece al reconectar
> - **Módulo Rust:** `chat`
> - **Prioridad:** Must Have

> **US-10** Como usuario quiero ver el historial de mis conversaciones para retomar negociaciones pendientes
> - **Criterio de aceptación:** La lista de chats muestra todas las conversaciones ordenadas por fecha del último mensaje, con indicador de mensajes no leídos (count > 0)
> - **Módulo Rust:** `chat`
> - **Prioridad:** Must Have

> **US-11** Como usuario quiero recibir mensajes en tiempo real sin recargar la página para una experiencia fluida
> - **Criterio de aceptación:** Al recibir un mensaje vía WebSocket, este aparece en la ventana de chat en menos de 100ms sin intervención del usuario
> - **Módulo Rust:** `chat`
> - **Prioridad:** Must Have

### Módulo `ratings`

> **US-12** Como comprador quiero valorar al vendedor tras completar una compra para contribuir a su reputación
> - **Criterio de aceptación:** Se permite puntuar de 1 a 5 estrellas con comentario opcional; solo una valoración por usuario por transacción; el `avg_rating` del valorado se recalcula automáticamente
> - **Módulo Rust:** `ratings`
> - **Prioridad:** Must Have

> **US-13** Como usuario quiero ver las valoraciones de otro usuario en su perfil público para decidir si confío en él
> - **Criterio de aceptación:** El perfil muestra promedio numérico, total de valoraciones y las últimas 10 con comentario y fecha
> - **Módulo Rust:** `ratings`
> - **Prioridad:** Must Have

### Módulo `geo`

> **US-14** Como vendedor quiero asociar una ubicación a mi anuncio para que aparezca a compradores cercanos
> - **Criterio de aceptación:** El anuncio almacena `location_lat` y `location_lon` en BD; la ubicación se muestra como nombre de ciudad, nunca como dirección exacta
> - **Módulo Rust:** `geo`
> - **Prioridad:** Must Have

> **US-15** Como comprador quiero filtrar anuncios por distancia máxima para ver solo productos accesibles desde mi zona
> - **Criterio de aceptación:** El filtro de radio (5, 10, 25, 50 km) excluye correctamente anuncios fuera del rango usando fórmula Haversine en la query SQL
> - **Módulo Rust:** `geo`
> - **Prioridad:** Must Have

### Módulo `favorites`

> **US-16** Como comprador quiero guardar anuncios en favoritos para revisarlos más tarde sin tener que buscarlos de nuevo
> - **Criterio de aceptación:** El botón alterna entre guardar y quitar (toggle); la lista de favoritos es accesible desde el perfil; si el anuncio se elimina, desaparece automáticamente de favoritos (ON DELETE CASCADE)
> - **Módulo Rust:** `favorites`
> - **Prioridad:** Must Have

### Módulo `payments`

> **US-17** Como comprador quiero pagar un anuncio con tarjeta vía Stripe para asegurar la transacción
> - **Criterio de aceptación:** Se crea un PaymentIntent en Stripe, se redirige a Stripe Checkout; tras pago exitoso la transacción queda en estado `paid` en BD y se notifica al vendedor
> - **Módulo Rust:** `payments`
> - **Prioridad:** Must Have

> **US-18** Como vendedor quiero recibir confirmación de pago para saber que la venta se ha completado
> - **Criterio de aceptación:** El webhook de Stripe actualiza el estado de la transacción a `paid` en BD; el vendedor ve la venta en su listado con el monto neto (descontada la comisión de plataforma)
> - **Módulo Rust:** `payments`
> - **Prioridad:** Must Have

---

## 6. Modelo de datos simplificado

8 entidades principales. Los nombres de tabla y campo siguen la convención `snake_case`. Todos los `id` son UUID v4. Timestamps en `TIMESTAMPTZ` (UTC).

### Tabla `users`

| Campo | Tipo | Descripción | Restricciones |
|-------|------|-------------|---------------|
| `id` | UUID | Identificador único | PK, gen aleatorio |
| `email` | VARCHAR(255) | Correo electrónico | UNIQUE, NOT NULL, **índice** |
| `password_hash` | VARCHAR(255) | Hash argon2id de la contraseña | NOT NULL |
| `display_name` | VARCHAR(100) | Nombre público visible | NOT NULL |
| `avatar_url` | TEXT | URL de foto de perfil en Cloudinary | NULL |
| `bio` | VARCHAR(500) | Descripción breve del usuario | NULL |
| `location_lat` | FLOAT8 | Latitud del usuario | NULL |
| `location_lon` | FLOAT8 | Longitud del usuario | NULL |
| `city` | VARCHAR(100) | Ciudad configurada | NULL |
| `avg_rating` | FLOAT4 | Promedio de valoraciones recibidas | NOT NULL, DEFAULT 0.0 |
| `rating_count` | INT4 | Total de valoraciones recibidas | NOT NULL, DEFAULT 0 |
| `role` | VARCHAR(20) | Rol: `user` o `admin` | NOT NULL, DEFAULT 'user' |
| `created_at` | TIMESTAMPTZ | Fecha de registro | NOT NULL, DEFAULT now() |
| `updated_at` | TIMESTAMPTZ | Última modificación | NOT NULL, DEFAULT now() |

### Tabla `listings`

| Campo | Tipo | Descripción | Restricciones |
|-------|------|-------------|---------------|
| `id` | UUID | Identificador único | PK |
| `seller_id` | UUID | Usuario propietario | FK → users(id), NOT NULL, **índice** |
| `title` | VARCHAR(150) | Título del anuncio | NOT NULL |
| `description` | TEXT | Descripción detallada | NOT NULL |
| `price` | NUMERIC(10,2) | Precio en EUR | NOT NULL, CHECK > 0 |
| `category` | VARCHAR(50) | Categoría del producto | NOT NULL, **índice** |
| `condition` | VARCHAR(20) | Estado físico: `new`, `like_new`, `used` | NOT NULL |
| `status` | VARCHAR(20) | Estado lógico: `active`, `sold`, `deleted` | NOT NULL, DEFAULT 'active', **índice** |
| `location_lat` | FLOAT8 | Latitud del anuncio | NOT NULL |
| `location_lon` | FLOAT8 | Longitud del anuncio | NOT NULL |
| `city` | VARCHAR(100) | Ciudad del anuncio | NOT NULL |
| `created_at` | TIMESTAMPTZ | Fecha de publicación | NOT NULL, DEFAULT now(), **índice** |
| `updated_at` | TIMESTAMPTZ | Última modificación | NOT NULL, DEFAULT now() |

### Tabla `listing_images`

| Campo | Tipo | Descripción | Restricciones |
|-------|------|-------------|---------------|
| `id` | UUID | Identificador único | PK |
| `listing_id` | UUID | Anuncio asociado | FK → listings(id) ON DELETE CASCADE, NOT NULL, **índice** |
| `image_url` | TEXT | URL en Cloudinary | NOT NULL |
| `position` | INT2 | Orden de la imagen (0 = principal) | NOT NULL, DEFAULT 0 |

### Tabla `conversations`

| Campo | Tipo | Descripción | Restricciones |
|-------|------|-------------|---------------|
| `id` | UUID | Identificador único | PK |
| `listing_id` | UUID | Anuncio vinculado | FK → listings(id), NOT NULL |
| `buyer_id` | UUID | Comprador que inicia el chat | FK → users(id), NOT NULL |
| `seller_id` | UUID | Vendedor del anuncio | FK → users(id), NOT NULL |
| `created_at` | TIMESTAMPTZ | Fecha de inicio | NOT NULL, DEFAULT now() |
| `updated_at` | TIMESTAMPTZ | Timestamp del último mensaje | NOT NULL, DEFAULT now(), **índice** |

UNIQUE constraint en (`listing_id`, `buyer_id`) — un solo chat por comprador por anuncio.

### Tabla `messages`

| Campo | Tipo | Descripción | Restricciones |
|-------|------|-------------|---------------|
| `id` | UUID | Identificador único | PK |
| `conversation_id` | UUID | Conversación padre | FK → conversations(id) ON DELETE CASCADE, NOT NULL, **índice** |
| `sender_id` | UUID | Autor del mensaje | FK → users(id), NOT NULL |
| `content` | TEXT | Texto del mensaje | NOT NULL, CHECK length > 0 |
| `is_read` | BOOL | Leído por el destinatario | NOT NULL, DEFAULT false |
| `created_at` | TIMESTAMPTZ | Fecha de envío | NOT NULL, DEFAULT now() |

### Tabla `transactions`

| Campo | Tipo | Descripción | Restricciones |
|-------|------|-------------|---------------|
| `id` | UUID | Identificador único | PK |
| `listing_id` | UUID | Anuncio comprado | FK → listings(id), NOT NULL, **índice** |
| `buyer_id` | UUID | Comprador | FK → users(id), NOT NULL |
| `seller_id` | UUID | Vendedor | FK → users(id), NOT NULL |
| `amount` | NUMERIC(10,2) | Monto total en EUR | NOT NULL, CHECK > 0 |
| `platform_fee` | NUMERIC(10,2) | Comisión de plataforma | NOT NULL, DEFAULT 0.00 |
| `stripe_payment_id` | VARCHAR(255) | ID del PaymentIntent de Stripe | UNIQUE, NULL |
| `status` | VARCHAR(20) | `pending`, `paid`, `completed`, `refunded` | NOT NULL, DEFAULT 'pending', **índice** |
| `created_at` | TIMESTAMPTZ | Fecha de la transacción | NOT NULL, DEFAULT now() |

### Tabla `ratings`

| Campo | Tipo | Descripción | Restricciones |
|-------|------|-------------|---------------|
| `id` | UUID | Identificador único | PK |
| `transaction_id` | UUID | Transacción asociada | FK → transactions(id), NOT NULL |
| `rater_id` | UUID | Quien emite la valoración | FK → users(id), NOT NULL |
| `rated_id` | UUID | Quien recibe la valoración | FK → users(id), NOT NULL, **índice** |
| `score` | INT2 | Puntuación de 1 a 5 | NOT NULL, CHECK BETWEEN 1 AND 5 |
| `comment` | VARCHAR(500) | Comentario opcional | NULL |
| `created_at` | TIMESTAMPTZ | Fecha de valoración | NOT NULL, DEFAULT now() |

UNIQUE constraint en (`transaction_id`, `rater_id`) — una sola valoración por parte por transacción.

### Tabla `favorites`

| Campo | Tipo | Descripción | Restricciones |
|-------|------|-------------|---------------|
| `id` | UUID | Identificador único | PK |
| `user_id` | UUID | Usuario que guarda | FK → users(id) ON DELETE CASCADE, NOT NULL, **índice** |
| `listing_id` | UUID | Anuncio guardado | FK → listings(id) ON DELETE CASCADE, NOT NULL |
| `created_at` | TIMESTAMPTZ | Fecha de guardado | NOT NULL, DEFAULT now() |

UNIQUE constraint en (`user_id`, `listing_id`) — un favorito por usuario por anuncio.

### Relaciones entre entidades

- **users → listings:** 1:N — Un usuario publica muchos anuncios (`seller_id`)
- **listings → listing_images:** 1:N — Un anuncio tiene varias imágenes; cascade delete
- **users + listings → conversations:** Un comprador abre una conversación por anuncio; relación ternaria
- **conversations → messages:** 1:N — Una conversación contiene muchos mensajes; cascade delete
- **listings → transactions:** 1:1 — Una sola venta por anuncio (el anuncio pasa a `sold`)
- **transactions → ratings:** 1:2 — Cada transacción genera hasta dos valoraciones (comprador ↔ vendedor)
- **users ↔ listings vía favorites:** N:M — Relación muchos-a-muchos a través de tabla intermedia

---

## 7. Requisitos no funcionales

| Categoría | Requisito | Métrica objetivo | Prioridad |
|-----------|-----------|-----------------|-----------|
| **Rendimiento** | Tiempo de respuesta de endpoints REST | P95 < 200ms para consultas simples; P95 < 500ms para búsqueda con filtros | Must Have |
| **Rendimiento** | Latencia de mensajes WebSocket | < 100ms entre envío y renderizado en el cliente | Must Have |
| **Rendimiento** | Tiempo de carga de página | First Contentful Paint < 1.5s con templates Askama pre-compilados | Should Have |
| **Seguridad** | Hashing de contraseñas | argon2id con parámetros OWASP (m=19456, t=2, p=1) | Must Have |
| **Seguridad** | Autenticación en endpoints protegidos | JWT validado vía middleware Axum en cada request; token expirado → HTTP 401 | Must Have |
| **Seguridad** | Datos de pago | Cero datos de tarjeta en nuestra BD; 100% delegado a Stripe Checkout | Must Have |
| **Seguridad** | Protección contra inyección SQL | Todas las queries usan parámetros bind de SQLx; cero concatenación de strings | Must Have |
| **Seguridad** | HTTPS en producción | Todos los endpoints servidos sobre TLS; cookies con flag `Secure` y `HttpOnly` | Must Have |
| **Escalabilidad** | Arquitectura modular por crates | Cada dominio en crate independiente; cambio en un módulo no fuerza recompilación completa | Should Have |
| **Escalabilidad** | Pool de conexiones a BD | SQLx con pool configurable por entorno (dev: 5, prod: 20 conexiones máx.) | Must Have |
| **Escalabilidad** | Búsqueda desacoplada | MeiliSearch como servicio externo indexado; si cae, fallback a query SQL LIKE | Should Have |
| **Mantenibilidad** | Arquitectura hexagonal | Separación ports/adapters: la lógica de dominio no importa Axum ni SQLx directamente | Should Have |
| **Mantenibilidad** | Tests de integración | Mínimo 1 test por endpoint Must Have usando `sqlx::test` con BD efímera | Should Have |
| **Mantenibilidad** | Migraciones versionadas | Todas las migraciones en `/migrations` con SQLx; aplicables con `sqlx migrate run` | Must Have |
| **Compatibilidad** | Navegadores soportados | Chrome 90+, Firefox 90+, Safari 15+, Edge 90+ | Must Have |
| **Compatibilidad** | Diseño responsive | Usable en viewports de 375px (móvil) a 1920px (desktop) con TailwindCSS breakpoints | Must Have |

---

## 8. Dependencias técnicas y externas

### Crates de Rust

| Crate | Versión aprox. | Propósito |
|-------|---------------|-----------|
| `axum` | 0.7.x | Framework web principal: routing, handlers, extractors, middleware |
| `askama` | 0.12.x | Templates HTML compilados y validados en tiempo de compilación |
| `sqlx` | 0.7.x | Queries SQL con validación en compilación, driver PostgreSQL, migraciones |
| `tokio` | 1.x | Runtime asíncrono requerido por Axum y todos los crates async |
| `jsonwebtoken` | 9.x | Generación y validación de tokens JWT (HS256) |
| `argon2` | 0.5.x | Hashing seguro de contraseñas con argon2id |
| `serde` / `serde_json` | 1.x | Serialización y deserialización de structs a JSON y viceversa |
| `tower-http` | 0.5.x | Middlewares HTTP: CORS, logging con tracing, compresión gzip |
| `tokio-tungstenite` | 0.21.x | WebSockets para chat en tiempo real sobre Tokio |
| `stripe-rust` | 0.34.x | Cliente Stripe: creación de PaymentIntents, gestión de webhooks |
| `meilisearch-sdk` | 0.27.x | Cliente MeiliSearch para indexación y búsqueda full-text |
| `uuid` | 1.x | Generación de UUIDs v4 para PKs de todas las entidades |
| `chrono` | 0.4.x | Manejo de fechas, timestamps y zonas horarias |
| `tracing` + `tracing-subscriber` | 0.1.x | Logging estructurado con niveles y contexto por request |

### Servicios externos

| Servicio | Criticidad | Uso | Fallback si no disponible |
|----------|-----------|-----|--------------------------|
| **PostgreSQL 15+** | Crítico (Must Have) | Base de datos principal; todo el modelo de datos | Ninguno — sin BD no hay aplicación |
| **Stripe** (modo test) | Crítico (Must Have) | Procesamiento de pagos; PaymentIntents y webhooks | Marcar transacciones como `paid` manualmente para demo |
| **MeiliSearch** | Crítico (Must Have) | Búsqueda full-text de anuncios con filtros combinables | Fallback a query SQL con `ILIKE` y filtros WHERE |
| **Cloudinary** | Alto (Must Have) | Almacenamiento y transformación de imágenes de anuncios | Almacenamiento local en `/static/uploads/` con servido estático |
| **Redis** | Medio (Should Have) | Caché de sesiones JWT y pub/sub para WebSockets multi-instancia | Sin caché; JWT stateless no lo requiere estrictamente |

> **Nota:** `redis-mcp` fue evaluado y descartado — no existe servidor MCP disponible para Redis. Redis como servicio de infraestructura permanece en la categoría "Should Have" con su fallback documentado.

### MCPs por fase de desarrollo

| MCP | Sprint | Uso concreto |
|-----|--------|-------------|
| `github-mcp` | Todos (0–3) | Gestión de repositorio, creación de PRs, issues y branches por feature |
| `postgres-mcp` | Sprint 1–2 | Verificación de esquema, ejecución de migraciones, validación de queries |
| `meilisearch-mcp` | Sprint 2 | Creación de índices, configuración de atributos filtrables y ordenables |
| `cloudinary-mcp` | Sprint 2 | Configuración de upload presets y transformaciones de imagen |
| `stripe-mcp` | Sprint 3 | Configuración de productos, precios y endpoints de webhook |
| `playwright-mcp` | Sprint 3 | Tests E2E automatizados de flujos críticos (registro → compra → valoración) |

### MCPs evaluados y descartados

| MCP | Motivo del descarte |
|-----|--------------------|
| `redis-mcp` | No existe servidor MCP disponible para Redis. La caché JWT es opcional (JWT stateless) y el chat no requiere pub/sub externo en esta versión. |
| `figma-mcp` | No existe servidor MCP disponible para Figma. El sistema de diseño es autocontenido en las skills `tailwind-patterns` y `askama-template`; no se necesita herramienta de prototipado externa. |

---

## 9. Riesgos y plan de mitigación

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|-----------|
| **Complejidad de Rust para generación con IA** — El borrow checker, lifetimes y el sistema de tipos estricto causan más errores de compilación que lenguajes dinámicos al generar código con agentes | Alta | Alto | Usar crates con APIs ergonómicas y abundante documentación; preferir `Clone` sobre lifetimes complejos; compilar incrementalmente cada 2-3 archivos; mantener un catálogo de patrones Axum validados que los agentes reutilicen |
| **Disponibilidad desigual del equipo** — 3 personas con horarios no alineados generan cuellos de botella en revisión y merges | Alta | Medio | Asignar módulos independientes por persona (ej: persona A = `users`+`listings`, persona B = `chat`+`ratings`, persona C = `search`+`payments`); usar GitHub Issues como fuente de verdad; daily asíncrono de 5 min en texto |
| **Integraciones externas que fallen o tarden** — Stripe, Cloudinary o MeiliSearch pueden requerir configuración manual, tener documentación ambigua o sufrir downtime | Media | Alto | Crear todas las cuentas y obtener API keys en Sprint 0 (día 0); implementar fallbacks para cada servicio (§8); tener un script `setup_services.sh` que valide conectividad con todos los servicios antes de cada sprint |
| **Scope creep** — Tentación de añadir features no planificadas porque "son fáciles" o "quedarían bien para la demo" | Alta | Alto | Este PRD es el contrato de alcance vinculante; cualquier adición Must Have requiere sacar otra Must Have; revisión de scope obligatoria al inicio de cada sprint; las Should Have solo se tocan si los 8 Must Have están verdes |
| **WebSockets en Rust** — La implementación de chat en tiempo real con `tokio-tungstenite` tiene curva de aprendizaje significativa y manejo complejo de conexiones concurrentes | Media | Medio | Implementar primero polling HTTP (GET `/chat/:id/messages?since=timestamp`) como fallback funcional que cumple el criterio de aceptación; WebSocket real como mejora progresiva en Sprint 2-3 |
| **Tiempo insuficiente para tests** — La presión del deadline de 1 semana puede sacrificar la calidad y cobertura de tests | Alta | Medio | Tests de integración solo para happy paths de los 8 Must Have; usar `sqlx::test` con BD efímera para velocidad; zero tests unitarios de lógica trivial; priorizar tests E2E con Playwright sobre tests unitarios |

---

## 10. Criterios de éxito de la entrega

### Criterios mínimos (aprobado)

Todas estas condiciones deben cumplirse el día de la entrega:

1. Un usuario puede registrarse con email/contraseña y recibir un JWT válido
2. Un usuario puede iniciar sesión y el JWT permite acceder a endpoints protegidos
3. Un vendedor puede crear un anuncio con al menos 1 imagen y este aparece en el listado público
4. Un vendedor puede editar y eliminar sus propios anuncios
5. Un comprador puede buscar anuncios por texto libre y filtrar por categoría y precio
6. Dos usuarios pueden intercambiar mensajes a través del chat vinculado a un anuncio
7. Un comprador puede completar un pago con Stripe (modo test) y la transacción queda registrada en BD
8. Un usuario puede valorar a otro tras una transacción completada (1-5 estrellas)
9. Los anuncios muestran ubicación (ciudad) y se pueden filtrar por distancia
10. Un usuario puede marcar y desmarcar anuncios como favoritos
11. El proyecto compila sin errores con `cargo build` y se ejecuta con `cargo run`
12. Existe documentación completa: PRD, arquitectura, y log de decisiones IA en `docs/ai_log/`

### Criterios de excelencia (sobresaliente)

Condiciones adicionales que elevan la calidad de la entrega:

1. El chat funciona en tiempo real con WebSockets (no solo polling HTTP)
2. Existe al menos 1 test de integración por cada uno de los 8 módulos Must Have
3. La aplicación está desplegada en un servidor accesible públicamente (URL funcional)
4. El diseño responsive funciona correctamente en móvil (viewport 375px) sin elementos rotos
5. Se implementan al menos 2 funcionalidades Should Have completas
6. El flujo E2E completo (registro → publicar → buscar → chatear → pagar → valorar) se puede demostrar en menos de 3 minutos en la presentación
7. Todos los commits del repositorio están generados por agentes IA con mensajes descriptivos
