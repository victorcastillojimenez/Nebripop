# Registro de Decisiones de IA — Generación del PRD (Nebripop)

**ID del Registro:** AI-LOG-001  
**Fecha:** 22 de mayo de 2026  
**Fase:** Fase 1 — Especificación y Definición de Producto  
**Agente:** Antigravity (Claude 3.5 Sonnet / Gemini 3.5 Flash)  
**Objetivo:** Generar el Product Requirements Document (PRD) completo y priorizado para el clon de Wallapop "Nebripop".

---

## 1. Contexto de Entrada y Restricciones

Se partió del análisis del archivo `wallapop-deep-research-report.md` (un reporte detallado sobre el producto Wallapop) y `project-context.md` (el contexto de la asignatura y equipo).

### Restricciones Críticas Evaluadas:
1. **Equipo:** 3 personas, disponibilidad desigual, 1 semana de desarrollo real.
2. **Stack:** 
   - **Backend:** Rust (Axum, SQLx, Tokio, PostgreSQL).
   - **Frontend:** Server-side templates con Askama + TailwindCSS via CDN + Vanilla JS.
     * *Nota de decisión:* Aunque `project-context.md` mencionaba Leptos (WASM), la skill específica de generación de PRD y las instrucciones explícitas del usuario priorizan Askama templates. Se optó por **Askama** para asegurar consistencia con la especificación y evitar la complejidad de hidratación/WASM en un desarrollo de 1 semana.
3. **Generación:** Código manual = cero, 100% generado por agentes IA.
4. **Metodología:** XP adaptado con sprints de 2 días.

---

## 2. Prompts y Flujo de Interacción

Debido al límite de tokens de contexto en la ventana de salida (64k max tokens de respuesta), la generación se dividió en fases lógicas iterativas para evitar respuestas truncadas y garantizar la máxima profundidad en cada sección:

* **Paso 1 (Secciones 1, 2, 3):** Resumen ejecutivo, objetivos (negocio, técnicos, académicos) y definición de actores con tabla de endpoints.
* **Paso 2 (Secciones 4, 5):** Definición de alcance MoSCoW estructurado y redacción de 18 User Stories funcionales con criterios de aceptación binarios.
* **Paso 3 (Secciones 6, 7):** Modelo de datos relacional (PostgreSQL) optimizado y Requisitos No Funcionales (NFRs) detallados.
* **Paso 4 (Secciones 8, 9, 10):** Dependencias de crates Rust, servicios externos (Stripe, Cloudinary, MeiliSearch), mapa de MCPs por sprint, matriz de riesgos y criterios de éxito.
* **Paso 5 (Consolidación):** Unión y formateo de cabeceras en `docs/PRD.md`.

---

## 3. Decisiones de Diseño de Producto y Arquitectura

### A. Priorización MoSCoW (Must Have / Out of Scope)
Se alineó la priorización con las restricciones temporales del equipo (1 semana):
- **Must Have (8 Features Core):** Auth JWT, CRUD de Anuncios, Búsqueda, Chat, Valoraciones, Geolocalización, Favoritos, Pagos Stripe.
- **Out of Scope Explícito:** Logística de transportistas (couriers), búsquedas con ML (imágenes), dashboards analíticos avanzados y soporte/helpdesk. Esto evita el *scope creep* y asegura que el MVP pueda ser completado en 1 semana por agentes de IA.

### B. Diseño del Modelo de Datos (8 Entidades)
Se limitó el modelo a **8 entidades** para mantener la consistencia física en PostgreSQL y evitar complejidades de joins masivos:
1. `users`: Gestión de cuentas, reputación e identidad.
2. `listings`: Catálogo de anuncios activos.
3. `listing_images`: Optimización de imágenes 1:N asociadas a Cloudinary.
4. `conversations`: Agrupador de chat único por par (comprador, listing) para simplificar queries.
5. `messages`: Mensajes individuales con timestamps para WebSockets.
6. `transactions`: Control de estado de pagos y escrow con Stripe (`pending`, `paid`, `completed`, `refunded`).
7. `ratings`: Reputación mutua vinculada a transacciones.
8. `favorites`: Tabla intermedia N:M para persistencia de lista de deseos.

### C. Mitigación de Riesgos Clave
- **Fallo de API Externa (Stripe/Cloudinary/Meili):** Se definieron mecanismos de fallback obligatorios en el backend Rust. Por ejemplo, si MeiliSearch falla, el backend delegará a queries SQL locales con `ILIKE`; si Cloudinary no responde, se almacenarán las imágenes localmente.
- **Curva de Rust + WebSockets:** Para asegurar el cumplimiento de la entrega, las User Stories del chat se diseñaron para permitir un fallback por polling HTTP como paso intermedio si la conexión WebSockets con `tokio-tungstenite` presenta problemas de concurrencia difíciles de depurar para la IA.

---

## 4. Trazabilidad del Artifact Generado

- **Archivo Destino:** [docs/PRD.md](file:///c:/AAmaster/Nebripop/docs/PRD.md)
- **Estado:** Finalizado y validado.
- **Tolerancia a fallos:** 100% de coherencia terminológica (nombres de crates, variables de base de datos y flujos de endpoints) respetada de principio a fin.
