---
name: docker-rust
description: Guía detallada y plantillas completas para empaquetar, contenerizar y desplegar Nebripop en entornos locales y de producción utilizando Docker y Docker Compose. Úsala siempre que el usuario mencione crear o configurar un Dockerfile, definir un docker-compose.yml, optimizar el tamaño de la imagen final con cargo-chef y distroless, configurar volúmenes persistentes, agregar healthchecks de bases de datos, inyectar variables de entorno mediante un archivo .env o ejecutar migraciones de SQLx en contenedores al inicio.
---

# Skill de Docker & Despliegue en Rust — Nebripop

Esta skill proporciona las directrices, plantillas y configuraciones exactas que los agentes de desarrollo deben seguir para contenerizar y orquestar el ecosistema de Nebripop (API Axum, PostgreSQL, Redis, y MeiliSearch) en Docker.

---

## Directrices de Calidad y Formato

Al utilizar esta skill, el agente debe regirse por los siguientes principios:

1. **Cero Placeholders**: Toda la configuración de Docker y Docker Compose debe entregarse 100% funcional. No se admiten comentarios de elisión.
2. **Eficiencia en Capas**: Es obligatorio implementar compilación multi-etapa con `cargo-chef` para evitar reconstruir dependencias externas si no hay cambios en `Cargo.lock`.
3. **Seguridad y Minimización**: La imagen final de ejecución debe utilizar **Distroless** como base. No debe contener gestores de paquetes ni shells (`bash`, `sh`), y el peso total comprimido debe ser **menor a 50MB**.
4. **Resiliencia de Red**: Los servicios deben tener políticas de reinicio y comprobaciones de estado (*healthchecks*) estrictas para evitar condiciones de carrera al arrancar.

---

## Dockerfile Multistage Optimizado (con `cargo-chef`)

Para compilar de forma eficiente en Rust y mantener una imagen final ultraligera (<50MB), se utiliza `gcr.io/distroless/cc-debian12` en la etapa final de ejecución. Esta imagen incluye `glibc`, `libgcc` y `openssl` (necesarios para librerías criptográficas de Rust y base de datos) pero carece de un sistema de archivos pesado, shells o herramientas vulnerables.

El archivo que el agente debe crear en la raíz del proyecto es el siguiente:

```dockerfile
# ==============================================================================
# ETAPA 1: Planificador (Planner)
# Genera el archivo recipe.json con las dependencias del workspace
# ==============================================================================
FROM lukemathwalker/cargo-chef:latest-rust-1.78.0 AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ==============================================================================
# ETAPA 2: Constructor (Builder)
# Compila las dependencias de forma aislada para aprovechar la caché de Docker
# ==============================================================================
FROM lukemathwalker/cargo-chef:latest-rust-1.78.0 AS builder
WORKDIR /app

# Copia la receta de dependencias calculada en la etapa 1
COPY --from=planner /app/recipe.json recipe.json

# Descarga y compila las dependencias externas (¡Se almacena en caché!)
RUN cargo chef cook --release --recipe-path recipe.json

# Copia el código fuente real del proyecto
COPY . .

# Compila el binario ejecutable del crate principal en modo release
RUN cargo build --release --bin api

# ==============================================================================
# ETAPA 3: Ejecución (Runtime)
# Imagen de producción ultraligera (<50MB) basada en Google Distroless CC
# ==============================================================================
FROM gcr.io/distroless/cc-debian12:latest AS runtime
WORKDIR /app

# Copia el binario compilado desde el builder
COPY --from=builder /app/target/release/api /app/nebripop-api

# Copia recursos estáticos o plantillas HTML Askama si son necesarios en runtime
# COPY --from=builder /app/static /app/static

# Expone el puerto por defecto de la aplicación
EXPOSE 8080

# Define variables de entorno para producción por defecto
ENV PORT=8080
ENV RUST_LOG=info

# Comando de arranque del contenedor
ENTRYPOINT ["/app/nebripop-api"]
```

---

## Orquestación Completa con Docker Compose (`docker-compose.yml`)

El orquestador une el backend en Rust con PostgreSQL, Redis y MeiliSearch. Todos los servicios cuentan con healthchecks que garantizan que el backend (`app`) arranque únicamente cuando todas sus dependencias estén completamente listas para recibir tráfico.

El archivo que el agente debe crear en la raíz del proyecto es el siguiente:

```yaml
version: '3.8'

services:
  # ----------------------------------------------------------------------------
  # Servicio A: Base de datos relacional (PostgreSQL)
  # ----------------------------------------------------------------------------
  db:
    image: postgres:15-alpine
    container_name: nebripop_db
    restart: always
    environment:
      POSTGRES_USER: ${DB_USER:-postgres}
      POSTGRES_PASSWORD: ${DB_PASSWORD:-postgres}
      POSTGRES_DB: ${DB_NAME:-nebripop}
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U $${POSTGRES_USER:-postgres} -d $${POSTGRES_DB:-nebripop}"]
      interval: 5s
      timeout: 5s
      retries: 5
      start_period: 10s

  # ----------------------------------------------------------------------------
  # Servicio B: Caché de sesiones y WebSocket pub/sub (Redis)
  # ----------------------------------------------------------------------------
  redis:
    image: redis:7-alpine
    container_name: nebripop_redis
    restart: always
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5

  # ----------------------------------------------------------------------------
  # Servicio C: Motor de búsqueda full-text (MeiliSearch)
  # ----------------------------------------------------------------------------
  meilisearch:
    image: getmeili/meilisearch:v1.5
    container_name: nebripop_meili
    restart: always
    environment:
      MEILI_ENV: ${MEILI_ENV:-development}
      MEILI_MASTER_KEY: ${MEILI_MASTER_KEY:-masterKeyNebripop2026}
      MEILI_NO_ANALYTICS: "true"
    ports:
      - "7700:7700"
    volumes:
      - meili_data:/meili_data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:7700/health"]
      interval: 5s
      timeout: 5s
      retries: 5

  # ----------------------------------------------------------------------------
  # Servicio D: Backend de Nebripop (Rust Axum + Askama/Leptos)
  # ----------------------------------------------------------------------------
  app:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: nebripop_app
    restart: always
    ports:
      - "${PORT:-8080}:${PORT:-8080}"
    environment:
      PORT: ${PORT:-8080}
      RUST_LOG: ${RUST_LOG:-info}
      DATABASE_URL: postgres://${DB_USER:-postgres}:${DB_PASSWORD:-postgres}@db:5432/${DB_NAME:-nebripop}
      REDIS_URL: redis://redis:6379
      MEILI_URL: http://meilisearch:7700
      MEILI_KEY: ${MEILI_MASTER_KEY:-masterKeyNebripop2026}
      JWT_SECRET: ${JWT_SECRET:-supersecretsecretseednebripop2026}
      STRIPE_SECRET_KEY: ${STRIPE_SECRET_KEY}
      CLOUDINARY_URL: ${CLOUDINARY_URL}
    depends_on:
      db:
        condition: service_healthy
      redis:
        condition: service_healthy
      meilisearch:
        condition: service_healthy
    volumes:
      # Volumen para almacenamiento local de imágenes (fallback de Cloudinary)
      - uploads_data:/app/static/uploads
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:${PORT:-8080}/health"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 15s

volumes:
  postgres_data:
    driver: local
  redis_data:
    driver: local
  meili_data:
    driver: local
  uploads_data:
    driver: local
```

---

## Gestión de Entornos mediante `.env`

Docker Compose lee por defecto el archivo `.env` ubicado en el mismo directorio. El agente debe guiar al usuario a crear un archivo `.env` local con los valores de desarrollo:

```ini
# Configuración del servidor
PORT=8080
RUST_LOG=debug

# Credenciales de Base de Datos PostgreSQL
DB_USER=postgres
DB_PASSWORD=postgres_secure_pass
DB_NAME=nebripop

# Clave maestra de MeiliSearch (Min 16 bytes para producción)
MEILI_MASTER_KEY=masterKeyNebripop2026
MEILI_ENV=development

# Credenciales del Negocio (Stripe & Cloudinary)
STRIPE_SECRET_KEY=sk_test_51... (Obtenida de Stripe Dashboard)
CLOUDINARY_URL=cloudinary://API_KEY:API_SECRET@CLOUD_NAME (Obtenida de Cloudinary)

# Firma de seguridad de tokens de sesión JWT
JWT_SECRET=seed_altamente_segura_y_aleatoria_para_jwt_2026
```

---

## Ejecución de Migraciones SQLx en Distroless al Arrancar

Dado que las imágenes **Distroless** no contienen terminales, `bash`, ni gestores de paquetes, **es imposible ejecutar comandos como `sqlx migrate run` dentro del contenedor de producción**.

La mejor práctica recomendada y el estándar en la comunidad de Rust consiste en **embeber las migraciones directamente en el binario** de Rust. De este modo, la propia aplicación ejecuta las migraciones de forma automática durante su inicialización asíncrona al arrancar el contenedor:

### Implementación en `crates/api/src/main.rs` (o punto de entrada del Backend):

El agente debe guiar la inserción del siguiente fragmento en la función de inicio del servidor:

```rust
use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Obtener la variable de entorno DATABASE_URL
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL debe estar configurada en el entorno");

    println!("Conectando a la base de datos PostgreSQL...");
    
    // 2. Crear el pool de conexiones
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;

    println!("Conexión establecida con éxito. Ejecutando migraciones SQLx...");

    // 3. Ejecutar migraciones embebidas al vuelo
    // El macro sqlx::migrate! busca automáticamente la carpeta /migrations en la raíz del proyecto
    // en tiempo de compilación y las incrusta en el binario final.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    println!("¡Migraciones aplicadas con éxito!");

    // 4. Inicializar y arrancar los routers de Axum
    let app = api::create_router(pool).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    
    println!("Servidor Axum corriendo en http://0.0.0.0:8080");
    axum::serve(listener, app).await?;

    Ok(())
}
```

---

## Comandos Útiles para Desarrollo Local

El agente debe proveer una guía de referencia de comandos rápidos para el control del entorno contenerizado:

- **Arrancar y construir los servicios en segundo plano**:
  ```bash
  docker compose up -d --build
  ```
- **Ver los registros (logs) del backend de Rust en tiempo real**:
  ```bash
  docker compose logs -f app
  ```
- **Verificar el estado de las comprobaciones de salud (Healthchecks)**:
  ```bash
  docker compose ps
  ```
- **Conectarse de forma interactiva al shell interactivo de PostgreSQL**:
  ```bash
  docker compose exec db psql -U postgres -d nebripop
  ```
- **Detener el entorno completo preservando los volúmenes**:
  ```bash
  docker compose down
  ```
- **Detener destruyendo por completo las bases de datos (Reinicio limpio)**:
  ```bash
  docker compose down -v
  ```
- **Reiniciar únicamente el backend en Rust (útil al recompilar código)**:
  ```bash
  docker compose restart app
  ```
