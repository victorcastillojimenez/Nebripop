# 🍿 Nebripop - El Wallapop Exclusivo para Nebrijanos

¡Bienvenido a **Nebripop**! Una plataforma web de compraventa C2C premium exclusiva para alumnos, docentes y miembros de la Universidad Antonio de Nebrija. Diseñada y construida con una arquitectura de software robusta, moderna y de máximo rendimiento en **Rust** utilizando un flujo de desarrollo ágil orquestado por agentes de Inteligencia Artificial de última generación.

---

## 🚀 Despliegue en Producción

La aplicación está desplegada en **Railway** y es completamente funcional en la siguiente dirección pública:

👉 **[https://nebripop-production.up.railway.app](https://nebripop-production.up.railway.app)**

> [!NOTE]
> El endpoint de salud operativa del backend `/health` responde en tiempo real confirmando el estado de la conexión a la base de datos y servicios externos:
> **`https://nebripop-production.up.railway.app/health`**

---

## 🛠️ Stack Tecnológico Completo

El sistema está estructurado mediante una pila tecnológica de nivel industrial para garantizar transacciones concurrentes seguras, búsquedas ultra-rápidas y un consumo de memoria mínimo:

### 🦀 Backend (Rust)
* **Axum**: Framework web asíncrono, modular e impulsado por tipos construido sobre Tokio.
* **SQLx & PostgreSQL**: Driver relacional 100% asíncrono que garantiza la seguridad de las consultas en tiempo de compilación. Incorpora extensiones geoespaciales (**PostGIS**) para geolocalización avanzada.
* **MeiliSearch**: Motor de búsqueda a texto completo con tolerancia a errores ortográficos, permitiendo encontrar anuncios con latencias inferiores al milisegundo.
* **JWT & Argon2id**: Autenticación segura sin estado mediante JSON Web Tokens firmados criptográficamente y hashing robusto de contraseñas con Argon2id.
* **Stripe**: Integración nativa para la gestión de pasarelas de pago, creación de Payment Intents y reconciliación de cobros mediante webhooks verificados.
* **Cloudinary**: Almacenamiento distribuido de imágenes, compresión automatizada y procesamiento visual en la nube.
* **WebSockets**: Canal bidireccional asíncrono en tiempo real para negociaciones instantáneas en el chat interno.

### 🎨 Frontend
* **Askama Templates**: Motor de plantillas compiladas directamente en código Rust (Type-safe HTML), lo que resulta en renderizados del lado del servidor (SSR) de velocidad ultra-rápida y cero sobrecarga de Javascript.
* **TailwindCSS**: Estética visual premium y responsiva diseñada de manera fluida y adaptada a dispositivos móviles.
* **Vanilla JavaScript**: Gestión de la interactividad del DOM, micro-animaciones dinámicas y reconexión resiliente del chat por WebSockets.

---

## 🏗️ Arquitectura del Sistema

Nebripop se adhiere rígidamente a los principios de **Arquitectura Hexagonal (Puertos y Adaptadores)** y **Clean Code**, garantizando un desacoplamiento completo entre la lógica del negocio puro y la infraestructura externa.

El proyecto está organizado mediante un **Cargo Workspace** con la siguiente jerarquía de crates:
* `crates/domain`: Entidades de negocio, Value Objects, Newtypes y reglas de negocio puras (sin dependencias externas).
* `crates/ports`: Puertos e interfaces que definen las firmas de persistencia, pasarelas de pago y búsqueda (inversión de dependencias).
* `crates/usecases`: Lógica pura de casos de uso (orquesta la llamada de puertos y el flujo de negocio).
* `crates/adapters`: Implementaciones de infraestructura concreta (PostgreSQL con SQLx, MeiliSearch, Stripe API, Cloudinary).
* `crates/api`: Orquestador principal, controladores HTTP, middleware de Axum, plantillas Askama y configuración del router.
* `crates/common`: Utilidades globales, parseos y estructuras compartidas de errores.

---

## 💻 Desarrollo y Setup Local

### Requisitos Previos
* [Rust Toolchain](https://www.rust-lang.org/) (Versión 1.75 o superior)
* [Docker & Docker Compose](https://www.docker.com/)

### Instrucciones de Inicio Rápido

1. **Clonar el repositorio**:
   ```bash
   git clone https://github.com/victorcastillojimenez/Nebripop.git
   cd Nebripop
   ```

2. **Levantar los servicios externos**:
   Puedes arrancar el stack de base de datos PostgreSQL:
   ```bash
   docker compose up -d postgres
   ```
   *Nota: Si deseas arrancar PostgreSQL + MeiliSearch al mismo tiempo, puedes ejecutar `docker compose up -d`.*

3. **Configurar el archivo de entorno**:
   Copia la plantilla de ejemplo y rellena los secretos locales y credenciales de Stripe/Cloudinary:
   ```bash
   cp .env.example .env
   ```

4. **Ejecutar las migraciones de base de datos**:
   ```bash
   cargo sqlx database setup
   ```

5. **Iniciar la API y el Portal Web en modo desarrollo**:
   ```bash
   cargo run -p api
   ```

La plataforma interactiva se iniciará en **`http://localhost:8080`**.

---

## 🧪 Ejecución de la Suite de Tests

El proyecto cuenta con una cobertura integral que incluye pruebas unitarias y robustos tests de integración concurrentes validados contra una base de datos PostgreSQL efímera gestionada automáticamente por `sqlx::test`.

Para ejecutar toda la suite de pruebas del workspace, escribe el siguiente comando:
```bash
cargo test --all
```

---

## 🔌 Endpoints Principales de la API y Rutas Web

### 🛡️ REST API (JSON - Bajo el prefijo `/api`)

| Método | Ruta | Autenticación | Descripción |
| :--- | :--- | :---: | :--- |
| **POST** | `/api/auth/register` | 🔓 Pública | Registro de un nuevo usuario |
| **POST** | `/api/auth/login` | 🔓 Pública | Autenticación y obtención de JWT |
| **POST** | `/api/auth/refresh` | 🔓 Pública | Renovación del token de sesión |
| **POST** | `/api/auth/logout` | 🔐 JWT | Invalidación del token de sesión |
| **GET** | `/api/users/:id` | 🔓 Pública | Perfil público y anuncios del usuario |
| **GET** | `/api/listings` | 🔓 Pública | Catálogo paginado de anuncios activos |
| **POST** | `/api/listings` | 🔐 JWT | Publicación de un nuevo anuncio |
| **GET** | `/api/listings/:id` | 🔓 Pública | Ficha detallada de un anuncio específico |
| **PUT** | `/api/listings/:id` | 🔐 JWT (Propietario) | Edición de un anuncio existente |
| **DELETE** | `/api/listings/:id` | 🔐 JWT (Propietario) | Eliminación lógica (soft-delete) del anuncio |
| **POST** | `/api/listings/:id/images` | 🔐 JWT (Propietario) | Subida de imágenes multimedia a Cloudinary |
| **GET** | `/api/search` | 🔓 Pública | Búsqueda por texto con MeiliSearch (SQL fallback) |
| **GET** | `/api/listings/nearby` | 🔓 Pública | Búsqueda geoespacial por coordenadas y radio (PostGIS) |
| **POST** | `/api/listings/:id/favorites` | 🔐 JWT | Añadir un anuncio a la lista de favoritos |
| **DELETE** | `/api/listings/:id/favorites` | 🔐 JWT | Quitar un anuncio de la lista de favoritos |
| **GET** | `/api/users/me/favorites` | 🔐 JWT | Listar anuncios favoritos del usuario autenticado |
| **POST** | `/api/listings/:id/ratings` | 🔐 JWT | Crear una valoración tras completar una compraventa |
| **GET** | `/api/users/:id/ratings` | 🔓 Pública | Listado de valoraciones y promedio de un usuario |
| **GET** | `/api/chat` | 🔐 JWT | Listar conversaciones activas |
| **POST** | `/api/chat` | 🔐 JWT | Iniciar una conversación sobre un anuncio |
| **GET** | `/api/chat/:id/messages` | 🔐 JWT | Obtener historial de mensajes (HTTP fallback) |
| **POST** | `/api/chat/:id/messages` | 🔐 JWT | Enviar mensaje en conversación vía REST |
| **GET** | `/api/chat/:id/ws` | 🔐 JWT | Upgrade de WebSocket para mensajería en tiempo real |
| **POST** | `/api/payments/intent` | 🔐 JWT | Crear Payment Intent de Stripe para compra directa |
| **GET** | `/api/payments/:id/status` | 🔐 JWT | Obtener estado de pago de una transacción |
| **POST** | `/api/payments/webhook` | 🔓 Webhook Stripe | Notificación de confirmación de cargo desde Stripe |

### 🖥️ Rutas Web (HTML rendered con Askama Templates)

| Ruta | Método | Acceso | Descripción |
| :--- | :---: | :---: | :--- |
| `/` | **GET** | 🔓 Libre | Home premium, categorías y anuncios destacados |
| `/listings` | **GET** | 🔓 Libre | Buscador y visualizador interactivo de anuncios |
| `/listings/:id` | **GET** | 🔓 Libre | Ficha del anuncio, geolocalización interactiva |
| `/listings/new` | **GET** | 🔐 Web Cookie | Formulario premium de publicación de anuncio |
| `/listings/create` | **POST** | 🔐 Web Cookie | Procesamiento de la creación del anuncio |
| `/search` | **GET** | 🔓 Libre | Resultados detallados de búsqueda |
| `/login` | **GET / POST**| 🔓 Libre | Formulario y procesamiento de login (Cookie-session integration)|
| `/register` | **GET / POST**| 🔓 Libre | Formulario y procesamiento de registro |
| `/logout` | **GET** | 🔐 Web Cookie | Cierre de sesión y limpieza de cookies |
| `/users/:id` | **GET** | 🔓 Libre | Visualización de perfil de otro usuario y sus artículos |
| `/me` | **GET** | 🔐 Web Cookie | Mi Perfil, historial de anuncios y favoritos |
| `/chat` | **GET** | 🔐 Web Cookie | Bandeja de entrada de mensajes |
| `/chat/:id` | **GET** | 🔐 Web Cookie | Interfaz interactiva de chat y negociación en vivo |
| `/payments/checkout/:id` | **GET** | 🔐 Web Cookie | Checkout premium integrado con Stripe Elements |
| `/payments/success` | **GET** | 🔐 Web Cookie | Pantalla de confirmación de compra completada |
| `/payments/error` | **GET** | 🔐 Web Cookie | Pantalla explicativa de error de transacción |
| `/health` | **GET** | 🔓 Libre | Endpoint simple de salud del sistema (API health checks) |

---

## 🤖 Sistema de Agentes de IA Utilizados

La totalidad del código fuente de Nebripop ha sido concebida, desarrollada, refactorizada y auditada de forma autónoma mediante una estructura coordinada de **agentes inteligentes** de dos plataformas complementarias:

### 🌌 Plataforma Antigravity (Alto Nivel — Arquitectura, UX y DevOps)
* **`architect-agent`**: Diseña e implementa la topología general en Rust, coordina la separación hexagonal del Cargo Workspace e impone la inversión de dependencias.
* **`reviewer-agent`**: El guardián de calidad. Audita la legibilidad del código, hace cumplir los principios SOLID y la filosofía *Clean Code*.
* **`security-agent`**: Audita la seguridad del backend Axum frente a vulnerabilidades OWASP Top 10, validación criptográfica de tokens y sanitización de inputs.
* **`uiux-agent`**: Diseña interfaces premium, gradientes interactivos, micro-animaciones del frontend e integra los estilos usando componentes TailwindCSS fluidos.
* **`devops-agent`**: Se encarga del empaquetado de contenedores Multistage, optimizaciones distroless y los pipelines CI/CD de automatización.
* **`docs-agent`**: El escritor técnico responsable de la trazabilidad y la coherencia metodológica de la memoria técnica y los manuales del proyecto.

### 💻 Plataforma OpenCode (Bajo Nivel — Generación y Lógica de Módulos)
* **`db-schema-agent`**: Diseña las tablas relacionales de la base de datos PostgreSQL, implementa PostGIS y crea los scripts de migración incrementales.
* **`auth-agent`**: Codifica los flujos de registro, login, hashing con Argon2id y extractores de autenticación JWT.
* **`codegen-listings-agent`**: Genera el dominio y los casos de uso para la persistencia de anuncios y la integración con la API de imágenes de Cloudinary.
* **`codegen-search-agent`**: Desarrolla la sincronización asíncrona de anuncios en MeiliSearch y define los motores híbridos de búsqueda.
* **`codegen-chat-agent`**: Desarrolla la gestión concurrente de WebSockets en Axum para soportar mensajería persistente en tiempo real.
* **`codegen-payments-agent`**: Modela las transacciones de compra, genera Payment Intents en Stripe y configura los manejadores idempotentes de webhooks.
* **`codegen-core-agent`**: Suministra la lógica de dominio centralizada, conversiones genéricas de tipos de datos y estructuración de crates.
* **`qa-agent`**: Genera la suite interactiva de pruebas de integración exhaustivas (`#[sqlx::test]`) y valida los flujos en los diversos módulos.
