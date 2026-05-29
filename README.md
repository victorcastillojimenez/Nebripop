# 🍿 Nebripop - El Wallapop Exclusivo para Nebrijanos

¡Bienvenido a **Nebripop**! Una plataforma premium de compraventa exclusiva para alumnos y miembros de la Universidad Antonio de Nebrija. Diseñada con una arquitectura de software robusta, moderna y ultraeficiente en **Rust**.

## 🚀 Despliegue en Producción

La aplicación está desplegada en **Railway** y es completamente funcional en la siguiente dirección pública:

👉 **[https://nebripop-production.up.railway.app](https://nebripop-production.up.railway.app)**

> [!NOTE]
> El endpoint de salud de la API `/health` devuelve el estado operativo del backend en tiempo real:
> `https://nebripop-production.up.railway.app/health`

---

## 🛠️ Stack Tecnológico

El proyecto está diseñado de forma modular utilizando tecnologías de alto rendimiento y seguridad de tipos:

### Backend (Rust)
* **Axum**: Framework web rápido, modular y asíncrono sobre Tokio.
* **SQLx & PostgreSQL**: Driver SQL moderno y 100% asíncrono con migraciones en tiempo de compilación y soporte espacial de geolocalización avanzada (**PostGIS**).
* **MeiliSearch**: Motor de búsqueda ultrarrápido a texto completo para localizar artículos instantáneamente.
* **JWT (JSON Web Tokens) & Hashing con Argon2id**: Autenticación segura y robusta para las cuentas de los usuarios.
* **Stripe**: Pasarela de pagos integrada para realizar transacciones seguras de compra/venta y reservar artículos directamente desde la plataforma.
* **Cloudinary**: Almacenamiento seguro y procesamiento optimizado de imágenes de anuncios en la nube.

### Frontend
* **Askama Templates**: Motores de plantillas compiladas en Rust (Type-safe HTML) para una velocidad de renderizado en microsegundos y cero sobrecarga.
* **TailwindCSS**: Diseño visual responsivo y estético premium de última generación.
* **Vanilla JavaScript & WebSockets**: Interactividad dinámica y chat interactivo en tiempo real para negociaciones fluidas entre compradores y vendedores.

---

## 🏗️ Arquitectura del Sistema

Nebripop está estructurado siguiendo los principios de la **Arquitectura Hexagonal (Puertos y Adaptadores)** y **Clean Code**, lo que desacopla la lógica de dominio de las dependencias externas (base de datos, búsqueda, APIs de terceros).

El proyecto se organiza mediante un **Cargo Workspace** con la siguiente separación de responsabilidades:
* `crates/domain`: Modelos de negocio y lógica pura (entidades, value objects, newtypes).
* `crates/ports`: Interfaces e inversión de control (traits de repositorios, pasarelas de pago y búsqueda).
* `crates/usecases`: Lógica de casos de uso puros y flujos de negocio.
* `crates/adapters`: Implementaciones de infraestructura (base de datos SQLx, búsqueda MeiliSearch, pasarelas Stripe, Cloudinary).
* `crates/api`: Endpoints REST, handlers de Axum, plantillas Askama y configuración del router principal.

---

## 💻 Desarrollo Local

### Requisitos previos
* [Rust](https://www.rust-lang.org/) (versión 1.75 o superior)
* [Docker & Docker Compose](https://www.docker.com/)

### Pasos para iniciar

1. **Clonar el repositorio**:
   ```bash
   git clone https://github.com/victorcastillojimenez/Nebripop.git
   cd Nebripop
   ```

2. **Levantar los servicios externos** (PostgreSQL + PostGIS, MeiliSearch):
   ```bash
   docker-compose up -d
   ```

3. **Configurar el archivo de entorno**:
   Copia el archivo `.env.example` a `.env` y rellena las credenciales locales de Stripe, Cloudinary y JWT.

4. **Ejecutar migraciones de la base de datos**:
   ```bash
   cargo sqlx database setup
   ```

5. **Iniciar la aplicación en desarrollo**:
   ```bash
   cargo run -p api
   ```

La plataforma estará disponible localmente en `http://localhost:8080`.
