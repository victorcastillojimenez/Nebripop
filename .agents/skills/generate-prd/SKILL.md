# skill: generate-prd

## Propósito
Generar un PRD completo, priorizado y técnicamente accionable en markdown,
a partir de un research de producto existente. El documento debe servir
simultáneamente como contrato de alcance del equipo, guía de implementación
para los agentes de desarrollo y evidencia académica del proceso de ingeniería.

## Contexto del proyecto
- Nombre del proyecto: Nebripop
- Equipo: 3 personas con disponibilidad desigual, 1 semana de desarrollo real
- Stack asignado: Rust (backend Axum + SQLx + Tokio), Askama (templates HTML server-side)
- Frontend: Axum + Askama + TailwindCSS via CDN + JavaScript vanilla
- Base de datos: PostgreSQL con migraciones SQLx
- Metodología: XP adaptado a IA, sprints de 2 días
- Restricción crítica: código manual = cero, todo generado por agentes IA
- Repositorio: github.com/victorcastillojimenez/Nebripop

## Rol del agente al ejecutar esta skill
Actúa como un Product Manager senior con experiencia en marketplaces C2C
(consumer-to-consumer). Razona sobre cada decisión de alcance teniendo en
cuenta las restricciones reales del equipo. Cuando priorices, justifica
brevemente por qué algo es Must Have y no Should Have, y viceversa.

## Output esperado
El PRD debe incluir obligatoriamente estas secciones en este orden:

### 1. Resumen ejecutivo
- Máximo 150 palabras
- Qué es el producto, para quién, qué problema resuelve
- Una frase que describa el alcance comprometido para esta entrega

### 2. Objetivos del proyecto
- Objetivos de negocio (qué debe conseguir el producto)
- Objetivos técnicos (qué debe demostrar la implementación)
- Objetivos académicos (qué valora el profesor según el enunciado)
- Formato: lista numerada, máximo 3 por categoría

### 3. Actores del sistema
- Tabla con: Nombre del actor | Descripción | Permisos principales
- Incluir al menos: Usuario anónimo, Usuario registrado (comprador),
  Usuario registrado (vendedor), Administrador/Moderador
- Para cada actor, indicar qué endpoints/funcionalidades puede acceder

### 4. Alcance del proyecto
Tabla MoSCoW con columnas:
Funcionalidad | Prioridad | Justificación | Módulo Rust asociado

- Must Have: máximo 8 funcionalidades. Son las que se implementan sí o sí
- Should Have: máximo 5, se implementan si el tiempo lo permite
- Could Have: máximo 4, solo si el equipo va muy adelantado
- Out of Scope: lista explícita de lo que NO se construye en esta entrega,
  con una frase justificando cada exclusión

### 5. User stories por módulo
Para cada módulo Must Have, entre 2 y 4 user stories.
Formato estricto:
> **US-[ID]** Como [actor] quiero [acción concreta] para [beneficio medible]
> - **Criterio de aceptación:** [condición verificable y medible]
> - **Módulo Rust:** [nombre del crate o módulo]
> - **Prioridad:** Must Have / Should Have / Could Have

### 6. Modelo de datos simplificado
- Una tabla markdown por entidad principal
- Columnas: Campo | Tipo | Descripción | Restricciones
- Relaciones entre entidades descritas en texto (no diagrama, eso va en
  architecture.md)
- Indicar qué campos son índices por razones de performance
- Máximo 8 entidades para mantener el scope realista

### 7. Requisitos no funcionales
Tabla con columnas: Categoría | Requisito | Métrica objetivo | Prioridad
Cubrir obligatoriamente estas categorías:
- Rendimiento (tiempos de respuesta de la API)
- Seguridad (autenticación, datos sensibles)
- Escalabilidad (diseño que permita crecer sin reescribir)
- Mantenibilidad (estructura del código, cobertura de tests)
- Compatibilidad (navegadores, dispositivos objetivo)

### 8. Dependencias técnicas y externas
- Lista de librerías Rust clave (crates) con su versión aproximada y propósito
- MCPs que se usarán durante el desarrollo y en qué fase
- Servicios externos necesarios (Stripe, Cloudinary, etc.) y si son
  opcionales o críticos para el Must Have
- axum — framework web principal, routing y handlers
- askama — templates HTML compilados y validados en tiempo de compilación
- sqlx — queries SQL con validación en compilación, driver PostgreSQL
- tokio — runtime asíncrono, requerido por Axum
- jsonwebtoken — generación y validación de JWT
- argon2 — hashing seguro de contraseñas
- serde / serde_json — serialización/deserialización
- tower-http — middlewares HTTP (CORS, logging, compresión)
- tokio-tungstenite — WebSockets para el chat en tiempo real
- stripe-rust — integración con Stripe para pagos
- meilisearch-sdk — cliente para búsqueda full-text

### 9. Riesgos y plan de mitigación
Tabla con columnas: Riesgo | Probabilidad | Impacto | Mitigación
Incluir obligatoriamente estos riesgos:
- Complejidad de Rust para generar código correcto con IA
- Disponibilidad desigual del equipo
- Integraciones externas que fallen o tarden en configurarse
- Scope creep (añadir features no planificadas)

### 10. Criterios de éxito de la entrega
- Lista de condiciones que, si se cumplen, el proyecto es un éxito
- Separar: criterios mínimos (aprobado) vs criterios de excelencia
- Debe ser verificable el día de la entrega

## Restricciones de generación
- Máximo 8 Must Have features, sin excepciones
- Cero funcionalidades que requieran ML, visión computacional o
  integraciones de logística con couriers reales
- Cada criterio de aceptación debe ser binario: o se cumple o no se cumple,
  nada de "debería funcionar bien"
- Los nombres de módulos Rust deben ser consistentes entre secciones
  (el mismo nombre en user stories, modelo de datos y dependencias)
- El lenguaje es técnico pero directo, sin relleno corporativo
- El documento completo no debe superar 2500 palabras para ser usable

## Formato de entrega
- Archivo: docs/PRD.md en la raíz del repositorio
- Encoding: UTF-8
- Todo el texto en español
- Los nombres de variables, campos de BD y módulos Rust en inglés (snake_case)
- Versión en el header: v1.0 — fecha de generación