---
name: askama-best-practices
description: Directrices de arquitectura, mejores prácticas y patrones de codificación para el frontend de Nebripop desarrollado con templates de Askama, TailwindCSS y JavaScript vanilla. Utiliza esta skill siempre que vayas a escribir, maquetar o modificar plantillas HTML, interactividad en el cliente (fetch, WebSockets) o flujos de renderizado desde Axum.
---

# Askama & UI Best Practices — Nebripop

Esta skill define los estándares de diseño, la estructura de herencia de plantillas (templates) y los patrones de integración de frontend para el marketplace **Nebripop**. La interfaz de usuario se genera en el servidor utilizando **Askama** (compilada de forma segura en Rust) y se dinamiza en el cliente mediante **TailwindCSS** (estética premium y responsive) y **JavaScript Vanilla** (interacción REST y tiempo real con WebSockets).

---

## 1. Estructura y Organización de Plantillas

Todos los archivos `.html` de plantillas deben almacenarse en el directorio `templates/` del crate que orquesta el servidor web (por defecto, `crates/api/templates/`).

### Estructura de Directorios en `templates/`
```
crates/api/templates/
├── base.html                 # Plantilla base (esqueleto HTML global)
├── errors/
│   └── error.html            # Renderizado de errores HTTP (404, 500)
├── listings/
│   ├── list.html             # Feed principal de anuncios
│   ├── detail.html           # Ficha de detalle de un anuncio
│   └── create.html           # Formulario para publicar anuncio
├── chat/
│   └── room.html             # Bandeja de mensajería (WebSockets)
├── search/
│   └── results.html          # Resultados de búsqueda y filtrado
└── users/
    ├── profile.html          # Perfil público e histórico de valoraciones
    └── edit.html             # Edición de perfil
```

### Herencia de Plantillas (Estructura de `base.html`)
`base.html` centraliza los estilos (Tailwind CSS), fuentes tipográficas premium (Google Fonts: *Inter* y *Outfit*) y bloques comunes como el navbar y el footer. Las páginas secundarias extienden de ella mediante la instrucción `{% extends "base.html" %}`.

```html
<!-- templates/base.html -->
<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}Nebripop{% endblock %}</title>
    <!-- TailwindCSS CDN (Premium con componentes modernos) -->
    <script src="https://cdn.tailwindcss.com"></script>
    <script>
        tailwind.config = {
            theme: {
                extend: {
                    colors: {
                        brand: {
                            50: '#f0fdf9',
                            500: '#0ea5e9', // Azul cielo premium
                            600: '#0284c7',
                            700: '#0369a1',
                        }
                    },
                    fontFamily: {
                        sans: ['Inter', 'sans-serif'],
                        display: ['Outfit', 'sans-serif'],
                    }
                }
            }
        }
    </script>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=Outfit:wght@400;500;600;700&display=swap" rel="stylesheet">
    <style>
        /* Clases auxiliares para estética premium (Glassmorphism) */
        .glass {
            background: rgba(255, 255, 255, 0.75);
            backdrop-filter: blur(12px);
            border: 1px solid rgba(255, 255, 255, 0.25);
        }
    </style>
    {% block head %}{% endblock %}
</head>
<body class="bg-slate-50 text-slate-800 font-sans min-h-screen flex flex-col">
    <!-- Navbar Común -->
    <header class="glass sticky top-0 z-50 shadow-sm transition-all duration-300">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
            <a href="/" class="font-display font-bold text-2xl text-brand-600 tracking-tight flex items-center gap-2">
                <span>🎈 Nebripop</span>
            </a>
            <!-- Autenticación Condicional -->
            <nav class="flex items-center gap-4">
                <a href="/search" class="text-slate-600 hover:text-brand-600 font-medium">Buscar</a>
                {% if current_user.is_some() %}
                    {% let user = current_user.as_ref().unwrap() %}
                    <a href="/listings/create" class="bg-brand-600 hover:bg-brand-700 text-white px-4 py-2 rounded-xl font-semibold shadow-md shadow-brand-500/20 transition-all">Subir Producto</a>
                    <a href="/chat" class="text-slate-600 hover:text-brand-600 font-medium">Mensajes</a>
                    <a href="/users/me" class="flex items-center gap-2 text-slate-700 hover:text-brand-600 font-semibold">
                        <span class="w-8 h-8 rounded-full bg-brand-500 text-white flex items-center justify-center font-bold">
                            {{ user.display_name.chars().next().unwrap() }}
                        </span>
                        <span class="hidden md:inline">{{ user.display_name }}</span>
                    </a>
                {% else %}
                    <a href="/login" class="text-slate-700 hover:text-brand-600 font-semibold">Iniciar Sesión</a>
                    <a href="/register" class="bg-brand-600 hover:bg-brand-700 text-white px-4 py-2 rounded-xl font-semibold">Registrarse</a>
                {% endif %}
            </nav>
        </div>
    </header>

    <!-- Área de Contenido Principal -->
    <main class="flex-grow max-w-7xl mx-auto w-full px-4 sm:px-6 lg:px-8 py-8">
        {% block content %}{% endblock %}
    </main>

    <!-- Footer Común -->
    <footer class="bg-slate-900 text-slate-400 py-8 border-t border-slate-800 mt-auto">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 text-center text-sm">
            <p>&copy; 2026 Nebripop. Clon funcional de Wallapop desarrollado íntegramente con Inteligencia Artificial.</p>
        </div>
    </footer>

    <!-- Bloque para Scripts de JS específicos -->
    {% block scripts %}{% endblock %}
</body>
</html>
```

---

## 2. Inyección de Datos desde Rust (`derive Template`)

En el controlador web de Axum, las páginas se renderizan devolviendo un struct que implementa `Template`. Askama realiza la validación de tipos y sintaxis del template en **tiempo de compilación**.

### Representación en Rust
Toda variable renderizada en la plantilla debe declararse en el struct.

```rust
use askama::Template;
use axum::{
    response::{IntoResponse, Html},
    http::StatusCode,
};
use serde::Serialize;

// DTO del usuario actual para la barra de navegación
#[derive(Serialize, Clone)]
pub struct UserNavbarDto {
    pub id: uuid::Uuid,
    pub display_name: String,
}

// Estructura de la plantilla para el listado de anuncios
#[derive(Template)]
#[template(path = "listings/list.html")] // Buscado en crates/api/templates/
pub struct ListingsListTemplate {
    pub current_user: Option<UserNavbarDto>, // Requerido por base.html
    pub listings: Vec<ListingSummaryDto>,   // Datos específicos
}

#[derive(Serialize)]
pub struct ListingSummaryDto {
    pub id: uuid::Uuid,
    pub title: String,
    pub price: String, // Formateado (ej. "120.00 €")
    pub city: String,
    pub first_image_url: String,
}

// Handler de Axum que renderiza la vista
pub async fn list_listings_handler(
    State(state): State<AppState>,
    claims: Option<Claims>, // Extraído opcionalmente (anónimo o logueado)
) -> Result<impl IntoResponse, AppError> {
    
    // 1. Resolver usuario de navegación si el token JWT existe
    let current_user = match claims {
        Some(token) => {
            let u = users::usecases::find_user_by_id(token.sub, &state.db).await?;
            u.map(|user| UserNavbarDto {
                id: user.id,
                display_name: user.display_name,
            })
        }
        None => None,
    };

    // 2. Cargar datos del catálogo
    let listings_domain = listings::usecases::get_active_listings(&state.db).await?;
    let listings = listings_domain.into_iter().map(|l| ListingSummaryDto {
        id: l.id,
        title: l.title,
        price: format!("{:.2} €", l.price),
        city: l.city,
        first_image_url: l.first_image_url.unwrap_or_else(|| "/static/img/placeholder.jpg".to_string()),
    }).collect();

    // 3. Crear el Template struct
    let template = ListingsListTemplate {
        current_user,
        listings,
    };

    // 4. Mapear a respuesta HTML con código de estado 200 OK
    Ok(Html(template.render().map_err(|e| AppError::Internal(e.to_string()))?))
}
```

---

## 3. Ejemplo de Extensión (`templates/listings/list.html`)

Así es como un archivo secundario hereda y sustituye los bloques definidos en `base.html`.

```html
{% extends "base.html" %}

{% block title %}Comprar y Vender Cerca de Ti — Nebripop{% endblock %}

{% block content %}
<div class="space-y-8">
    <!-- Banner de Bienvenida -->
    <div class="relative overflow-hidden rounded-3xl bg-gradient-to-r from-brand-600 to-indigo-600 p-8 md:p-12 text-white shadow-xl shadow-brand-500/10">
        <h1 class="font-display font-bold text-3xl md:text-5xl mb-4 leading-tight">Encuentra chollos de segunda mano con Nebripop</h1>
        <p class="text-brand-100 text-lg max-w-xl">Publica tus anuncios de forma gratuita y conéctate con compradores y vendedores locales seguros.</p>
    </div>

    <!-- Rejilla de Anuncios -->
    <div>
        <h2 class="font-display font-semibold text-2xl text-slate-800 mb-6">Productos Destacados</h2>
        {% if listings.is_empty() %}
            <div class="glass rounded-2xl p-12 text-center text-slate-500">
                <p class="text-lg">No hay anuncios activos disponibles en este momento.</p>
                <a href="/listings/create" class="mt-4 inline-block bg-brand-600 text-white px-6 py-2 rounded-xl">¡Sé el primero en publicar!</a>
            </div>
        {% else %}
            <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6">
                {% for item in listings %}
                    <div class="bg-white rounded-2xl overflow-hidden shadow-sm hover:shadow-md hover:-translate-y-1 transition-all duration-300 border border-slate-100 flex flex-col">
                        <div class="relative aspect-square bg-slate-100">
                            <img src="{{ item.first_image_url }}" alt="{{ item.title }}" class="w-full h-full object-cover">
                            <span class="absolute bottom-3 left-3 bg-white text-brand-600 font-bold px-3 py-1 rounded-lg text-lg shadow-sm">
                                {{ item.price }}
                            </span>
                        </div>
                        <div class="p-4 flex-grow flex flex-col justify-between">
                            <div>
                                <h3 class="font-semibold text-slate-800 line-clamp-1 mb-1">{{ item.title }}</h3>
                                <p class="text-xs text-slate-400 flex items-center gap-1">📍 {{ item.city }}</p>
                            </div>
                            <a href="/listings/{{ item.id }}" class="mt-4 block w-full text-center bg-slate-50 hover:bg-brand-50 hover:text-brand-600 text-slate-600 py-2 rounded-xl font-medium text-sm transition-colors">
                                Ver Detalles
                            </a>
                        </div>
                    </div>
                {% endfor %}
            </div>
        {% endif %}
    </div>
</div>
{% endblock %}
```

---

## 4. Interactividad con Vanilla JS (Fetch API y Formularios)

Para evitar refrescos inútiles y ofrecer una experiencia interactiva sin añadir frameworks pesados, utilizaremos **JavaScript Vanilla** interceptando los formularios e interactuando con la API REST de Axum usando `fetch()`.

### Ejemplo: Marcar como Favorito (`listings/detail.html`)
```html
<!-- Detalle del Anuncio -->
<button id="btn-favorite" 
        data-listing-id="{{ listing.id }}" 
        class="flex items-center gap-2 px-4 py-2 border border-slate-200 hover:border-brand-500 rounded-xl transition-all duration-300">
    <span id="fav-icon" class="text-slate-400 text-lg">🤍</span>
    <span id="fav-text" class="font-medium text-slate-600">Guardar Favorito</span>
</button>

{% block scripts %}
<script>
    document.addEventListener("DOMContentLoaded", () => {
        const btn = document.getElementById("btn-favorite");
        const icon = document.getElementById("fav-icon");
        const text = document.getElementById("fav-text");

        btn.addEventListener("click", async () => {
            const listingId = btn.getAttribute("data-listing-id");
            const jwt = localStorage.getItem("nebripop_jwt");

            if (!jwt) {
                alert("Debes iniciar sesión para añadir a favoritos.");
                window.location.href = "/login";
                return;
            }

            try {
                // Petición a la API usando Fetch
                const response = await fetch(`/api/v1/listings/${listingId}/favorite`, {
                    method: 'POST',
                    headers: {
                        'Authorization': `Bearer ${jwt}`,
                        'Content-Type': 'application/json'
                    }
                });

                if (response.ok) {
                    const data = await response.json(); // Retorna {"favorited": true} o {"favorited": false}
                    if (data.favorited) {
                        icon.innerText = "❤️";
                        text.innerText = "En Favoritos";
                        btn.classList.add("bg-red-50", "border-red-200");
                    } else {
                        icon.innerText = "🤍";
                        text.innerText = "Guardar Favorito";
                        btn.classList.remove("bg-red-50", "border-red-200");
                    }
                } else if (response.status === 401) {
                    alert("Sesión expirada. Por favor, inicia sesión nuevamente.");
                    window.location.href = "/login";
                }
            } catch (err) {
                console.error("Error al procesar favoritos:", err);
            }
        });
    });
</script>
{% endblock %}
```

---

## 5. Integración de WebSockets en el Cliente para Chat en Tiempo Real

El chat en tiempo real (`US-11` del PRD) se gestiona de forma interactiva en el navegador abriendo una conexión WebSocket contra el servidor Axum.

```html
<!-- templates/chat/room.html -->
{% extends "base.html" %}

{% block content %}
<div class="h-[600px] glass rounded-3xl overflow-hidden flex flex-col shadow-lg border border-slate-100">
    <div class="p-4 border-b border-slate-100 bg-slate-50/50 flex items-center justify-between">
        <h2 class="font-display font-semibold text-lg text-slate-800">Mensajes con {{ seller_name }}</h2>
    </div>
    
    <!-- Contenedor de Mensajes -->
    <div id="messages-container" class="flex-grow p-6 overflow-y-auto space-y-4 bg-slate-50/20">
        <!-- El historial inicial cargado desde Askama loop -->
        {% for msg in history %}
            {% if msg.is_me %}
                <div class="flex justify-end">
                    <div class="bg-brand-600 text-white px-4 py-2 rounded-2xl rounded-tr-none max-w-xs md:max-w-md shadow-sm">
                        <p class="text-sm">{{ msg.content }}</p>
                    </div>
                </div>
            {% else %}
                <div class="flex justify-start">
                    <div class="bg-white text-slate-800 px-4 py-2 rounded-2xl rounded-tl-none max-w-xs md:max-w-md shadow-sm border border-slate-100">
                        <p class="text-sm">{{ msg.content }}</p>
                    </div>
                </div>
            {% endif %}
        {% endfor %}
    </div>

    <!-- Input de envío -->
    <div class="p-4 border-t border-slate-100 bg-white flex gap-4">
        <input type="text" id="chat-input" placeholder="Escribe tu mensaje..." class="flex-grow border border-slate-200 rounded-xl px-4 focus:ring-2 focus:ring-brand-500 outline-none text-sm">
        <button id="btn-send" class="bg-brand-600 hover:bg-brand-700 text-white px-6 py-2 rounded-xl font-semibold transition-colors">
            Enviar
        </button>
    </div>
</div>
{% endblock %}

{% block scripts %}
<script>
    document.addEventListener("DOMContentLoaded", () => {
        const container = document.getElementById("messages-container");
        const input = document.getElementById("chat-input");
        const btnSend = document.getElementById("btn-send");
        
        // Hacer auto-scroll al fondo al iniciar
        container.scrollTop = container.scrollHeight;

        const conversationId = "{{ conversation_id }}";
        const jwt = localStorage.getItem("nebripop_jwt");

        // 1. Establecer conexión de WebSocket
        const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        const wsUrl = `${protocol}//${window.location.host}/api/v1/chat/ws?token=${jwt}&conversation_id=${conversationId}`;
        const ws = new WebSocket(wsUrl);

        // 2. Escuchar mensajes del WebSocket (Servidor -> Cliente)
        ws.onmessage = (event) => {
            const data = JSON.parse(event.data); // Estructura: {"sender_id": "...", "content": "..."}
            appendMessage(data.content, data.is_me);
        };

        // 3. Función auxiliar para renderizar mensaje nuevo
        function appendMessage(content, isMe) {
            const wrapper = document.createElement("div");
            wrapper.className = isMe ? "flex justify-end" : "flex justify-start";
            
            const bubble = document.createElement("div");
            bubble.className = isMe 
                ? "bg-brand-600 text-white px-4 py-2 rounded-2xl rounded-tr-none max-w-xs md:max-w-md shadow-sm"
                : "bg-white text-slate-800 px-4 py-2 rounded-2xl rounded-tl-none max-w-xs md:max-w-md shadow-sm border border-slate-100";
            
            bubble.innerHTML = `<p class="text-sm">${content}</p>`;
            wrapper.appendChild(bubble);
            container.appendChild(wrapper);
            container.scrollTop = container.scrollHeight; // Auto-scroll
        }

        // 4. Enviar mensaje por el socket
        function sendMessage() {
            const content = input.value.trim();
            if (!content) return;

            // Enviar payload en formato JSON estructurado
            ws.send(JSON.stringify({ content: content }));
            
            appendMessage(content, true); // Render inmediato
            input.value = "";
        }

        btnSend.addEventListener("click", sendMessage);
        input.addEventListener("keypress", (e) => { if (e.key === 'Enter') sendMessage(); });
    });
</script>
{% endblock %}
```

---

## 6. Manejo de Errores y Páginas HTML de Error (404, 500)

En Nebripop, cuando ocurra un fallo en un handler web SSR (por ejemplo, buscar un detalle de anuncio inexistente), no debemos retornar una respuesta JSON cruda, sino renderizar una página de error amigable con el código de estado HTTP adecuado.

### Plantilla de Error (`templates/errors/error.html`)
```html
{% extends "base.html" %}

{% block title %}Error — Nebripop{% endblock %}

{% block content %}
<div class="max-w-md mx-auto py-12 text-center space-y-6">
    <div class="text-8xl">⚠️</div>
    <h1 class="font-display font-bold text-4xl text-slate-800">{{ status_code }}</h1>
    <h2 class="font-semibold text-xl text-slate-600">{{ error_title }}</h2>
    <p class="text-slate-400 text-sm leading-relaxed">{{ error_message }}</p>
    <a href="/" class="inline-block bg-brand-600 hover:bg-brand-700 text-white font-semibold px-6 py-3 rounded-xl shadow-md transition-all">
        Volver a la Página de Inicio
    </a>
</div>
{% endblock %}
```

### Handler de Renderizado de Errores (Rust)
```rust
use askama::Template;
use axum::response::{Html, IntoResponse};
use axum::http::StatusCode;

#[derive(Template)]
#[template(path = "errors/error.html")]
struct ErrorTemplate {
    current_user: Option<UserNavbarDto>,
    status_code: u16,
    error_title: String,
    error_message: String,
}

// Función auxiliar para retornar respuestas HTML de error en el servidor
pub fn render_html_error(
    status: StatusCode,
    title: &str,
    message: &str,
    user: Option<UserNavbarDto>
) -> Response {
    let template = ErrorTemplate {
        current_user: user,
        status_code: status.as_u16(),
        error_title: title.to_string(),
        error_message: message.to_string(),
    };

    match template.render() {
        Ok(html_content) => (status, Html(html_content)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("<h1>Error Interno Crítico</h1><p>No se pudo procesar la plantilla de error.</p>")
        ).into_response()
    }
}
```

---

## 7. Patrones Correctos vs. Incorrectos

### A. Herencia y Repetición de Esqueleto HTML

❌ **Incorrecto (Duplicación del código básico del esqueleto, CSS CDN y navbar. Dificulta mantenibilidad)**
```html
<!-- templates/listings/detail.html -->
<!DOCTYPE html>
<html>
<head>
    <title>Detalle del Anuncio</title>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body>
    <header>Navbar repetido con código duplicado...</header>
    <main>Detalle de {{ title }}</main>
</body>
</html>
```

✅ **Correcto (Heredar de `base.html` y sobreescribir los bloques lógicos)**
```html
<!-- templates/listings/detail.html -->
{% extends "base.html" %}

{% block title %}{{ listing.title }} — Nebripop{% endblock %}

{% block content %}
<div class="grid grid-cols-1 md:grid-cols-2 gap-8">
    <!-- Contenido específico de la ficha de detalle -->
</div>
{% endblock %}
```

---

### B. Mutaciones por Formulario Tradicional vs. Captura Fetch

❌ **Incorrecto (Formulario tradicional para mutaciones secundarias que fuerza recarga completa de página afectando la fluidez)**
```html
<!-- Formulario que obliga al navegador a refrescar y cambiar de pantalla para un simple favorito -->
<form action="/listings/{{ listing.id }}/favorite" method="POST">
    <button type="submit" class="bg-slate-100 p-2">Guardar Favorito</button>
</form>
```

✅ **Correcto (Evitar refresco con JavaScript e interactuar asíncronamente con la API REST del backend)**
```html
<button id="btn-favorite" class="bg-slate-100 p-2">Guardar Favorito</button>

{% block scripts %}
<script>
    document.getElementById("btn-favorite").addEventListener("click", async () => {
        const response = await fetch("/api/v1/listings/{{ listing.id }}/favorite", { method: "POST" });
        if (response.ok) {
            // Dinamizar la UI cambiando estilos localmente de inmediato
        }
    });
</script>
{% endblock %}
```

---

## 8. Las 10 Reglas Críticas de Askama para Nebripop

1. **Herencia Obligatoria**: Toda página pública o privada del frontend de Nebripop debe heredar de `base.html` de forma estricta mediante `{% extends "base.html" %}`.
2. **Inyección Fuerte de Tipos**: Pasa la información a renderizar en las vistas empaquetada exclusivamente en structs fuertemente tipados de Rust que implementen `#[derive(Template)]`. Evita la conversión a tipos genéricos dinámicos en Rust.
3. **Control de Sesiones (`current_user`)**: Todos los structs de plantillas que dependan del navbar deben contener el campo `current_user: Option<UserNavbarDto>` para adaptar visualmente la navegación (login/registro frente a perfil/mensajes).
4. **Tailwind CSS Centralizado**: Integra Tailwind CSS exclusivamente en la cabecera `<head>` de `base.html` y restringe los elementos visuales a la paleta corporativa y tipografía establecida (*Inter* y *Outfit*) para dar coherencia estética.
5. **Acciones Dinámicas via Fetch API**: Para peticiones de mutación de datos en caliente (favoritos, publicar valoraciones, eliminar listados, cambios de perfil), utiliza JavaScript con `fetch()` y cabeceras JSON en lugar de usar llamadas directas a formularios HTML.
6. **WebSockets para Alta Interactividad**: El chat (`US-11` del PRD) debe correr scripts Javascript asíncronos en el cliente para abrir un canal WebSocket interactivo e inmediato contra el orquestador Axum.
7. **Consistencia de IDs para Testing Automatizado**: Todo elemento interactivo del DOM (botones de acción, formularios, inputs, alertas) debe tener un atributo `id` único y semántico en inglés (`btn-favorite`, `btn-send`, `chat-input`) para facilitar los tests E2E con Playwright.
8. **Uso Seguro de la Macro de Escape**: Confía siempre en el escapado automático de Askama frente a vulnerabilidades Cross-Site Scripting (XSS). Solo utiliza el filtro `|safe` cuando renderices contenido sanitizado explícitamente y de fuentes totalmente seguras.
9. **Renderizado de Errores SSR**: En caso de fallo SSR en handlers de páginas de servidor, mapea los problemas de datos o páginas no encontradas (404) hacia el template unificado `errors/error.html` retornando el código HTTP correcto.
10. **Seguridad y Almacenamiento Local (LocalStorage)**: Almacena de forma segura el JWT en `localStorage` tras el login o registro del usuario, y añádelo automáticamente en la cabecera `Authorization: Bearer <token>` de cada petición Fetch o WebSocket iniciada en el cliente.
