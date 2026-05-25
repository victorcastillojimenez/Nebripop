---
name: github-actions-rust
description: Guía completa y plantilla lista para configurar el pipeline CI/CD en GitHub Actions para el proyecto Rust (Axum + Leptos + SQLx) de Nebripop. Úsala siempre que el usuario mencione integración continua, despliegue continuo, GitHub Actions, configurar workflows, testeo en la nube, optimizar tiempos de build en CI, compilar clippy, configurar base de datos en GitHub Actions o desplegar automáticamente en Railway/Fly.io.
---

# Skill de GitHub Actions & CI/CD para Rust — Nebripop

Esta skill proporciona las directrices, plantillas y configuraciones exactas que los agentes deben seguir para configurar e implementar el pipeline de Integración Continua y Despliegue Continuo (CI/CD) de Nebripop en GitHub Actions.

---

## Directrices de Calidad y Formato

Al utilizar esta skill, el agente debe regirse por los siguientes principios:

1. **Cero Placeholders**: Toda la configuración de GitHub Actions debe entregarse lista para su uso. No se admiten comentarios tipo `# Añadir otros pasos aquí`.
2. **Eficiencia Técnica**: Es obligatorio implementar caché de dependencias y de la carpeta `target/` para que el pipeline tarde menos de 3 minutos.
3. **Seguridad Estricta**: No se deben incrustar credenciales hardcodeadas. Se debe utilizar el almacén de secretos de GitHub (`secrets.*`).
4. **Resiliencia ante Fallos**: El pipeline debe fallar de inmediato ante advertencias de compilación (`clippy`), fallos en tests, o problemas de formateo.

---

## Estructura Completa del Pipeline CI/CD

El pipeline CI/CD de Nebripop debe constar de 4 fases o trabajos (*jobs*) encadenados:

```mermaid
graph TD
    fmt[Format Check] --> clippy[Linter: Clippy]
    clippy --> test[Test: Postgres Container]
    test --> build[Build: Axum + Leptos]
    build --> deploy[Deploy: Railway/Fly.io]
```

### 1. Fase de Formateo y Linter (`lint`)
- **Formateo**: Ejecuta `cargo fmt --all -- --check` para verificar la homogeneidad del código.
- **Linter**: Corre `cargo clippy --workspace --all-targets -- -D warnings` para forzar que cualquier *warning* o sugerencia de Clippy detenga y falle el pipeline inmediatamente.

### 2. Fase de Pruebas Unitarias y de Integración (`test`)
- Requiere una base de datos PostgreSQL real y efímera corriendo en un **Service Container** dentro de GitHub Actions.
- Requiere la instalación optimizada de `sqlx-cli` para aplicar las migraciones antes de ejecutar los tests.
- Ejecuta `cargo test --workspace`.

### 3. Fase de Construcción (`build`)
- Compila el proyecto completo en modo *release* para garantizar la compilación libre de errores y preparar los artefactos desplegables.

### 4. Fase de Despliegue Continuo (`deploy`)
- **Filtro de Rama**: Este trabajo solo debe ejecutarse si la compilación y los tests fueron exitosos, y si el disparador es un *push* o *merge* directo en la rama principal (`main`).
- Realiza el despliegue automático a plataformas en la nube como **Railway** o **Fly.io** usando sus respectivas herramientas CLI.

---

## Optimización de Tiempos mediante Caché

Para evitar compilar el compilador de Rust y todas las dependencias en cada ejecución del workflow:
- **`Swatinem/rust-cache`**: Debe ser el paso inicial tras hacer el checkout del repositorio. Se encarga de gestionar de forma inteligente la caché de `~/.cargo` y de la carpeta `./target` de Rust a nivel de llaves de caché vinculadas al archivo `Cargo.lock`.

---

## Configuración de PostgreSQL Efímero y SQLx

Para ejecutar las pruebas que dependen de la persistencia de SQLx sin conexión externa:
1. **Contenedor de Servicio**: Configurar `postgres:15` en el workflow exponiendo el puerto `5432` y definiendo usuario, contraseña y base de datos de testeo.
2. **Instalación Rápida de SQLx CLI**: Compilar `sqlx-cli` completo tarda más de 5 minutos en Actions. Debe instalarse **desactivando características innecesarias** y habilitando solo `postgres`:
   ```bash
   cargo install sqlx-cli --no-default-features --features postgres
   ```
   *Nota: Se recomienda cachear el binario de `sqlx` en `~/.cargo/bin` para acelerar futuras ejecuciones.*
3. **Migración de Datos**: Antes de correr los tests, se debe ejecutar:
   ```bash
   sqlx database setup
   # o alternativamente:
   sqlx migrate run
   ```

---

## Gestión de Secretos en GitHub

El agente debe instruir al equipo de desarrollo para que agregue los siguientes secretos en **Settings -> Secrets and variables -> Actions** del repositorio de GitHub:

| Secreto | Uso en el Pipeline / Aplicación |
|---------|--------------------------------|
| `DATABASE_URL` | Cadena de conexión a Postgres de producción (usada para despliegue y validación offline de SQLx). |
| `STRIPE_SECRET_KEY` | Clave API secreta en modo de prueba de Stripe. |
| `CLOUDINARY_URL` | URL de configuración de la API de Cloudinary. |
| `JWT_SECRET` | Firma criptográfica de los tokens JWT de sesión. |
| `RAILWAY_STATIC_API_TOKEN` | Token de autenticación de Railway (si se usa Railway). |
| `FLY_API_TOKEN` | Token de autenticación de Fly.io (si se usa Fly.io). |

---

## Archivo de Configuración Completo: `.github/workflows/ci.yml`

El archivo que el agente debe crear en el repositorio es el siguiente:

```yaml
name: Nebripop CI/CD Pipeline

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always
  DATABASE_URL: postgres://postgres:postgres@localhost:5432/nebripop_test
  STRIPE_SECRET_KEY: ${{ secrets.STRIPE_SECRET_KEY }}
  CLOUDINARY_URL: ${{ secrets.CLOUDINARY_URL }}
  JWT_SECRET: ${{ secrets.JWT_SECRET }}

jobs:
  lint:
    name: Lint & Formatting
    runs-on: ubuntu-latest
    steps:
      - name: Checkout Código
        uses: actions/checkout@v3

      - name: Instalar Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Caché de Cargo
        uses: Swatinem/rust-cache@v2

      - name: Verificar Formateo (rustfmt)
        run: cargo fmt --all -- --check

      - name: Análisis Estático (clippy)
        run: cargo clippy --workspace --all-targets -- -D warnings

  test:
    name: Integration & Unit Tests
    runs-on: ubuntu-latest
    needs: lint
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_USER: postgres
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: nebripop_test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - name: Checkout Código
        uses: actions/checkout@v3

      - name: Instalar Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Caché de Cargo
        uses: Swatinem/rust-cache@v2

      - name: Caché de SQLx CLI
        id: cache-sqlx
        uses: actions/cache@v3
        with:
          path: ~/.cargo/bin/sqlx
          key: ${{ runner.os }}-sqlx-${{ hashFiles('**/Cargo.lock') }}

      - name: Instalar SQLx CLI (si no está en caché)
        if: steps.cache-sqlx.outputs.cache-hit != 'true'
        run: cargo install sqlx-cli --no-default-features --features postgres

      - name: Ejecutar Migraciones SQLx
        run: sqlx database setup

      - name: Ejecutar Tests
        run: cargo test --workspace

  build:
    name: Build Release Binary
    runs-on: ubuntu-latest
    needs: test
    steps:
      - name: Checkout Código
        uses: actions/checkout@v3

      - name: Instalar Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Caché de Cargo
        uses: Swatinem/rust-cache@v2

      - name: Compilar Proyecto (Release)
        run: cargo build --workspace --release

  deploy:
    name: Deploy Continuous Delivery
    runs-on: ubuntu-latest
    needs: build
    if: github.ref == 'refs/heads/main' && github.event_name == 'push'
    steps:
      - name: Checkout Código
        uses: actions/checkout@v3

      # Opción A: Despliegue en Railway
      - name: Instalar Railway CLI
        run: npm install -g @railway/cli

      - name: Desplegar en Railway
        run: railway deploy --detach
        env:
          RAILWAY_TOKEN: ${{ secrets.RAILWAY_STATIC_API_TOKEN }}

      # Opción B: Despliegue en Fly.io (Alternativa comentada por seguridad)
      # - name: Instalar Flyctl
      #   uses: superfly/flyctl-actions/setup-flyctl@master
      # - name: Desplegar en Fly.io
      #   run: flyctl deploy --local-only
      #   env:
      #     FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
```

---

## Configuración de Protección de Rama en GitHub

El agente debe guiar al usuario para blindar la rama `main` en la interfaz de GitHub siguiendo estos pasos:

1. Entra al repositorio en GitHub.
2. Ve a **Settings** (Configuración) $\rightarrow$ **Branches** (Ramas).
3. Haz clic en **Add branch protection rule** (Añadir regla de protección de rama).
4. En **Branch name pattern**, escribe `main`.
5. Activa las siguientes opciones críticas:
   - **Require a pull request before merging**: Fuerza a que todo cambio pase por una propuesta de revisión en lugar de subirse directamente.
   - **Require status checks to pass before merging**: Bloquea el merge si el pipeline falla.
     - En la barra de búsqueda de *checks*, busca y selecciona `Lint & Formatting`, `Integration & Unit Tests`, y `Build Release Binary`.
6. Haz clic en **Create** para guardar la configuración.

---

## Badge de Estado en el README

Para mostrar el estado de la build directamente en el README del repositorio, se debe agregar el siguiente código markdown en la cabecera del archivo `README.md`:

```markdown
[![Nebripop CI/CD Pipeline](https://github.com/<owner>/<repo>/actions/workflows/ci.yml/badge.svg)](https://github.com/<owner>/<repo>/actions/workflows/ci.yml)
```
*(Nota: El agente debe reemplazar `<owner>` y `<repo>` por el nombre de usuario y el repositorio de GitHub reales del proyecto).*
