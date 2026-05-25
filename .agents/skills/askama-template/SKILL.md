---
name: askama-template
description: Directrices de arquitectura, mejores prácticas y plantillas HTML completas para la interfaz de usuario de Nebripop. Úsala siempre que vayas a escribir, maquetar o modificar plantillas de Askama, estructurar herencias de plantillas base, formularios reactivos con Vanilla JS o componentes visuales de catálogo.
---

# Askama Template Library & Components — Nebripop UI

Esta skill define la biblioteca de componentes interactivos y plantillas HTML reutilizables para el frontend de **Nebripop**. Su objetivo es proporcionar referencias de código completas, seguras ante inyecciones (XSS) y optimizadas estéticamente con **Tailwind CSS** y **JavaScript Vanilla**.

---

## 1. Integración Rust -> Askama (Tipado Fuerte)

Para inyectar datos de forma segura desde los controladores de Axum hacia las plantillas compiladas de Askama, declararemos structs estructurados que implementen `#[derive(Template)]`.

```rust
use askama::Template;

// DTO de sesión para el Navbar
pub struct UserSessionDto {
    pub display_name: String,
    pub avatar_url: Option<String>,
}

// Struct de datos para renderizar el feed principal
#[derive(Template)]
#[template(path = "listings/list.html")]
pub struct ListingsListTemplate {
    pub current_user: Option<UserSessionDto>, // navbar
    pub flash_success: Option<String>,        // mensaje flash de éxito
    pub flash_error: Option<String>,          // mensaje flash de error
    pub listings: Vec<ListingCardDto>,        // feed de tarjetas
    pub page_info: common::PageResult<()>,     // metadatos de paginación
}
```

---

## 2. Código HTML Completo: `base.html` (Plantilla Maestra)

El archivo `base.html` centraliza el esqueleto responsivo de Nebripop. Incorpora Tailwind CSS vía CDN, fuentes de Google Fonts (*Inter* y *Outfit*), soporte para notificaciones flotantes (Alertas Flash), navbar interactiva móvil/desktop con control de sesión y pie de página.

```html
<!-- crates/api/templates/base.html -->
<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="Nebripop - El marketplace de segunda mano definitivo para la comunidad universitaria de la Nebrija.">
    <title>{% block title %}Nebripop | Comprar y vender segunda mano{% endblock %}</title>
    
    <!-- Tailwind CSS y Google Fonts -->
    <script src="https://cdn.tailwindcss.com"></script>
    <script>
        tailwind.config = {
            theme: {
                extend: {
                    colors: {
                        brand: {
                            50: '#f0fdfa',
                            100: '#ccfbf1',
                            500: '#13c1ac', // Verde Turquesa Wallapop
                            600: '#0f9f8f',
                            700: '#0d7c70',
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
        .glass {
            background: rgba(255, 255, 255, 0.85);
            backdrop-filter: blur(12px);
            border: 1px solid rgba(255, 255, 255, 0.3);
        }
    </style>
    {% block head %}{% endblock %}
</head>
<body class="bg-slate-50 text-slate-800 font-sans min-h-screen flex flex-col antialiased">

    <!-- Mensajes Flash Renderizados Dinámicamente -->
    <div id="flash-container" class="fixed top-20 right-4 z-50 space-y-3 pointer-events-none">
        {% if flash_success.is_some() %}
            <div class="flash-alert bg-emerald-500 text-white px-5 py-3.5 rounded-2xl shadow-xl border border-emerald-400 flex items-center gap-3 transition-all duration-500 transform translate-x-0 pointer-events-auto">
                <span class="text-xl">✅</span>
                <span class="font-semibold text-sm">{{ flash_success.as_ref().unwrap() }}</span>
            </div>
        {% endif %}
        {% if flash_error.is_some() %}
            <div class="flash-alert bg-red-500 text-white px-5 py-3.5 rounded-2xl shadow-xl border border-red-400 flex items-center gap-3 transition-all duration-500 transform translate-x-0 pointer-events-auto">
                <span class="text-xl">⚠️</span>
                <span class="font-semibold text-sm">{{ flash_error.as_ref().unwrap() }}</span>
            </div>
        {% endif %}
    </div>

    <!-- Cabecera / Navbar -->
    <header class="glass sticky top-0 z-40 w-full shadow-sm">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between gap-4">
            
            <!-- Logo -->
            <a href="/" class="font-display font-bold text-2xl text-brand-500 flex items-center gap-1.5 transition-transform hover:scale-[1.02]">
                <span class="text-3xl">🎈</span>
                <span class="tracking-tight text-slate-800">Nebri<span class="text-brand-500">pop</span></span>
            </a>

            <!-- Buscador Desktop -->
            <div class="hidden md:flex flex-grow max-w-md">
                <form action="/search" method="GET" class="w-full relative">
                    <input type="text" name="q" placeholder="¿Qué estás buscando hoy?" 
                           class="w-full h-10 pl-4 pr-10 rounded-full border border-slate-200 focus:border-brand-500 focus:ring-4 focus:ring-brand-100 outline-none text-sm bg-slate-50 focus:bg-white text-slate-700 transition-all">
                    <button type="submit" class="absolute right-3.5 top-2.5 text-slate-400 hover:text-brand-500">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
                    </button>
                </form>
            </div>

            <!-- Navegación Sesión -->
            <nav class="flex items-center gap-4">
                <a href="/search" class="md:hidden p-2 text-slate-500 hover:text-brand-500 transition-colors" aria-label="Buscar">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
                </a>

                {% if current_user.is_some() %}
                    {% let user = current_user.as_ref().unwrap() %}
                    <!-- Subir Producto -->
                    <a href="/listings/create" class="hidden sm:flex items-center gap-1.5 bg-brand-500 hover:bg-brand-600 text-white px-4 py-2 rounded-2xl font-semibold shadow-md shadow-brand-500/10 hover:shadow-lg transition-all text-sm">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path></svg>
                        <span>Subir Anuncio</span>
                    </a>

                    <!-- Chats -->
                    <a href="/chat" class="relative p-2 text-slate-500 hover:text-brand-500 hover:bg-slate-50 rounded-xl transition-all" aria-label="Chats">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path></svg>
                        <span id="unread-dot" class="hidden absolute top-1.5 right-1.5 w-2.5 h-2.5 bg-brand-500 rounded-full ring-2 ring-white"></span>
                    </a>

                    <!-- Perfil -->
                    <a href="/users/me" class="flex items-center gap-2 group">
                        <div class="w-8 h-8 rounded-full bg-brand-100 text-brand-600 font-bold flex items-center justify-center border border-brand-200 group-hover:bg-brand-500 group-hover:text-white transition-all text-sm">
                            {{ user.display_name.chars().next().unwrap() }}
                        </div>
                        <span class="hidden md:inline text-sm font-semibold text-slate-700 group-hover:text-brand-500 transition-colors">{{ user.display_name }}</span>
                    </a>
                {% else %}
                    <a href="/login" class="text-sm font-semibold text-slate-600 hover:text-brand-500 px-3 py-2 rounded-xl transition-colors">Iniciar Sesión</a>
                    <a href="/register" class="text-sm font-semibold bg-brand-500 hover:bg-brand-600 text-white px-4 py-2 rounded-2xl shadow-sm hover:shadow-md transition-all">Registrarse</a>
                {% endif %}
            </nav>
        </div>
    </header>

    <!-- Contenido Principal -->
    <main class="flex-grow max-w-7xl mx-auto w-full px-4 sm:px-6 lg:px-8 py-8">
        {% block content %}{% endblock %}
    </main>

    <!-- Footer -->
    <footer class="bg-slate-900 text-slate-400 py-8 border-t border-slate-800">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 text-center text-sm">
            <p>&copy; 2026 Nebripop. Todos los derechos reservados. Desarrollado con Rust + Askama.</p>
        </div>
    </footer>

    <!-- Desvanecimiento automático de Alertas Flash -->
    <script>
        document.addEventListener("DOMContentLoaded", () => {
            const alerts = document.querySelectorAll(".flash-alert");
            alerts.forEach(alert => {
                setTimeout(() => {
                    alert.classList.add("opacity-0", "translate-x-10");
                    setTimeout(() => alert.remove(), 500);
                }, 4000);
            });
        });
    </script>
    {% block scripts %}{% endblock %}
</body>
</html>
```

---

## 3. Código HTML Completo: `listing-card.html` (Componente de Catálogo)

La tarjeta representa un anuncio en los feeds y búsquedas de Nebripop. Incorpora interactividad dinámica para guardar favoritos usando `fetch()` y control de clases adaptadas al estado físico del producto.

```html
<!-- crates/api/templates/components/listing-card.html -->
<div class="group relative bg-white rounded-3xl overflow-hidden border border-slate-100 shadow-sm hover:shadow-xl hover:-translate-y-1.5 transition-all duration-300 flex flex-col h-full">
    
    <!-- Imagen y Badges Flotantes -->
    <div class="relative aspect-[4/3] w-full overflow-hidden bg-slate-50 border-b border-slate-100">
        <img src="{{ item.image_url }}" 
             alt="{{ item.title }}" 
             class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
             loading="lazy">
        
        <!-- Precio Flotante -->
        <div class="absolute top-3 left-3 bg-white/95 backdrop-blur-sm px-3.5 py-1 rounded-2xl shadow-md border border-slate-100 flex items-center justify-center">
            <span class="font-display font-bold text-slate-800 text-base md:text-lg">{{ item.price }}</span>
        </div>

        <!-- Botón Asíncrono de Favoritos -->
        <button class="absolute top-3 right-3 w-8 h-8 rounded-full bg-white/90 backdrop-blur-sm hover:bg-white flex items-center justify-center text-slate-400 hover:text-red-500 shadow-md border border-slate-100 transition-all active:scale-90 btn-favorite-toggle" 
                data-listing-id="{{ item.id }}"
                data-is-favorited="{% if item.is_favorited %}true{% else %}false{% endif %}"
                aria-label="Guardar Favorito">
            <svg class="w-4 h-4 fill-current {% if item.is_favorited %}text-red-500{% else %}text-slate-400{% endif %}" viewBox="0 0 24 24">
                <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/>
            </svg>
        </button>

        <!-- Badge de Estado Físico (PRD 6.2) -->
        <div class="absolute bottom-3 left-3 flex gap-1.5">
            {% if item.condition == "new" %}
                <span class="bg-emerald-500/95 backdrop-blur-sm text-white text-[10px] font-bold tracking-wider uppercase px-2.5 py-0.5 rounded-lg shadow-sm">Nuevo</span>
            {% else if item.condition == "like_new" %}
                <span class="bg-brand-500/95 backdrop-blur-sm text-white text-[10px] font-bold tracking-wider uppercase px-2.5 py-0.5 rounded-lg shadow-sm">Como Nuevo</span>
            {% else %}
                <span class="bg-amber-500/95 backdrop-blur-sm text-white text-[10px] font-bold tracking-wider uppercase px-2.5 py-0.5 rounded-lg shadow-sm">Usado</span>
            {% endif %}
        </div>
    </div>

    <!-- Cuerpo de Información -->
    <div class="p-4 flex-grow flex flex-col justify-between gap-3 bg-white">
        <div class="space-y-1">
            <!-- Título del Anuncio -->
            <h3 class="font-display font-semibold text-slate-800 text-sm md:text-base line-clamp-1 group-hover:text-brand-500 transition-colors">
                {{ item.title }}
            </h3>
            
            <!-- Ubicación y Distancia Dinámica (PRD 5.2) -->
            <p class="text-xs text-slate-400 flex items-center gap-1">
                <svg class="w-3.5 h-3.5 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>
                <span>{{ item.city }}</span>
                {% if item.distance_km.is_some() %}
                    <span class="text-slate-200">•</span>
                    <span class="text-brand-600 font-semibold">a {{ item.distance_km.unwrap() }} km</span>
                {% endif %}
            </p>
        </div>

        <!-- Enlace Ficha -->
        <a href="/listings/{{ item.id }}" 
           class="block w-full text-center bg-slate-50 hover:bg-brand-50 text-slate-600 hover:text-brand-600 py-2.5 rounded-2xl font-bold text-xs md:text-sm transition-all duration-200">
            Ver detalles del producto
        </a>
    </div>
</div>
```

---

## 4. Componentes Reutilizables Auxiliares

### A. Reputación por Estrellas (`rating-stars`)
Muestra la media de valoraciones de un usuario de forma visual utilizando iconos SVG de estrellas coloreadas según el score medio (de 1 a 5).

```html
<!-- Componente: Valoración (users/profile.html) -->
<div class="flex items-center gap-1" aria-label="Valoración media de {{ rating }} sobre 5">
    {% for i in 1..5 %}
        <svg class="w-4 h-4 {% if i <= rating %}text-amber-400{% else %}text-slate-200{% endif %} fill-current" viewBox="0 0 24 24">
            <path d="M12 17.27L18.18 21l-1.64-7.03L22 9.24l-7.19-.61L12 2 9.19 8.63 2 9.24l5.46 4.73L5.82 21z"/>
        </svg>
    {% endfor %}
    <span class="text-xs text-slate-400 font-medium ml-1">({{ reviews_count }})</span>
</div>
```

### B. Avatar de Usuario Adaptativo (`user-avatar`)
Genera un avatar visual. Si no hay imagen de Cloudinary, genera dinámicamente un círculo con color corporativo y la primera inicial en mayúscula.

```html
<!-- Componente: Avatar -->
{% if user.avatar_url.is_some() %}
    <img src="{{ user.avatar_url.as_ref().unwrap() }}" alt="{{ user.display_name }}" class="w-10 h-10 rounded-full object-cover ring-2 ring-slate-100">
{% else %}
    <div class="w-10 h-10 rounded-full bg-brand-100 text-brand-600 font-bold border border-brand-200 flex items-center justify-center uppercase ring-2 ring-slate-100 text-sm">
        {{ user.display_name.chars().next().unwrap() }}
    </div>
{% endif %}
```

---

## 5. Paginación Visual de Resultados

Para controlar el feed de catálogos y búsquedas basándonos en el struct `PageResult<T>` del crate `common`, maquetaremos un pie de paginación fluida.

```html
<!-- Componente: Paginador (search/results.html) -->
{% if page_info.total > page_info.limit %}
<div class="flex items-center justify-between border-t border-slate-100 px-4 py-6 sm:px-6 mt-8">
    <div class="flex flex-1 justify-between sm:hidden">
        <a href="?page={{ page_info.page - 1 }}&q={{ query }}" 
           class="inline-flex items-center rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 {% if page_info.page == 1 %}pointer-events-none opacity-50{% endif %}">Anterior</a>
        <a href="?page={{ page_info.page + 1 }}&q={{ query }}" 
           class="inline-flex items-center rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 {% if page_info.items.len() < page_info.limit %}pointer-events-none opacity-50{% endif %}">Siguiente</a>
    </div>
    <div class="hidden sm:flex sm:flex-1 sm:items-center sm:justify-between">
        <div>
            <p class="text-sm text-slate-500">
                Mostrando <span class="font-semibold text-slate-800">{{ page_info.items.len() }}</span> de <span class="font-semibold text-slate-800">{{ page_info.total }}</span> resultados
            </p>
        </div>
        <div>
            <nav class="isolate inline-flex -space-x-px rounded-xl shadow-sm bg-white border border-slate-200 p-1 gap-1" aria-label="Paginación">
                <!-- Anterior -->
                <a href="?page={{ page_info.page - 1 }}&q={{ query }}" class="inline-flex items-center rounded-lg px-2.5 py-1.5 text-slate-500 hover:bg-slate-50 {% if page_info.page == 1 %}pointer-events-none opacity-50{% endif %}">
                    <span class="sr-only">Anterior</span>
                    <svg class="h-5 h-5" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M12.79 5.23a.75.75 0 01-.02 1.06L8.832 10l3.938 3.71a.75.75 0 11-1.04 1.08l-4.5-4.25a.75.75 0 010-1.08l4.5-4.25a.75.75 0 011.06.02z" clip-rule="evenodd" /></svg>
                </a>
                
                <!-- Número Actual -->
                <span class="inline-flex items-center rounded-lg bg-brand-500 px-4 py-1.5 text-sm font-semibold text-white">{{ page_info.page }}</span>
                
                <!-- Siguiente -->
                <a href="?page={{ page_info.page + 1 }}&q={{ query }}" class="inline-flex items-center rounded-lg px-2.5 py-1.5 text-slate-500 hover:bg-slate-50 {% if page_info.items.len() < page_info.limit %}pointer-events-none opacity-50{% endif %}">
                    <span class="sr-only">Siguiente</span>
                    <svg class="h-5 h-5" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z" clip-rule="evenodd" /></svg>
                </a>
            </nav>
        </div>
    </div>
</div>
{% endif %}
```

---

## 6. Formularios con Validación Client-Side (Vanilla JS)

Para agilizar el proceso de creación de anuncios (`listings/create.html`) antes de iniciar peticiones HTTP que consuman datos del servidor, validaremos de forma estricta los inputs mediante JavaScript en el navegador.

```html
<!-- Formulario: listings/create.html -->
<form id="form-create-listing" class="max-w-xl mx-auto bg-white p-8 rounded-3xl border border-slate-100 shadow-sm space-y-6">
    <div class="space-y-1.5">
        <label for="input-title" class="text-xs font-bold uppercase tracking-wider text-slate-500">Título del Anuncio</label>
        <input type="text" id="input-title" required minlength="5" maxlength="80" 
               class="w-full h-11 px-4 rounded-xl border border-slate-200 outline-none text-sm transition-all focus:border-brand-500 focus:ring-4 focus:ring-brand-100 bg-slate-50 focus:bg-white text-slate-700">
        <span id="error-title" class="hidden text-xs text-red-500 font-medium"></span>
    </div>

    <div class="space-y-1.5">
        <label for="input-description" class="text-xs font-bold uppercase tracking-wider text-slate-500">Descripción Detallada</label>
        <textarea id="input-description" required minlength="10" maxlength="5000" rows="4"
                  class="w-full p-4 rounded-xl border border-slate-200 outline-none text-sm transition-all focus:border-brand-500 focus:ring-4 focus:ring-brand-100 bg-slate-50 focus:bg-white text-slate-700"></textarea>
        <span id="error-description" class="hidden text-xs text-red-500 font-medium"></span>
    </div>

    <button type="submit" class="w-full bg-brand-500 hover:bg-brand-600 text-white font-semibold py-3 rounded-2xl transition-all shadow-md">Publicar Producto</button>
</form>

{% block scripts %}
<script>
    document.addEventListener("DOMContentLoaded", () => {
        const form = document.getElementById("form-create-listing");
        const title = document.getElementById("input-title");
        const description = document.getElementById("input-description");

        const errTitle = document.getElementById("error-title");
        const errDesc = document.getElementById("error-description");

        form.addEventListener("submit", (e) => {
            let isValid = true;

            // 1. Validar Título
            if (title.value.trim().length < 5) {
                errTitle.innerText = "El título debe tener al menos 5 caracteres.";
                errTitle.classList.remove("hidden");
                title.classList.add("border-red-500", "focus:ring-red-100");
                isValid = false;
            } else {
                errTitle.classList.add("hidden");
                title.classList.remove("border-red-500");
            }

            // 2. Validar Descripción
            if (description.value.trim().length < 10) {
                errDesc.innerText = "La descripción debe tener al menos 10 caracteres.";
                errDesc.classList.remove("hidden");
                description.classList.add("border-red-500", "focus:ring-red-100");
                isValid = false;
            } else {
                errDesc.classList.add("hidden");
                description.classList.remove("border-red-500");
            }

            if (!isValid) {
                e.preventDefault(); // Detener el envío del formulario
            }
        });
    });
</script>
{% endblock %}
```

---

## 7. Las 10 Reglas Críticas de Plantillas para Nebripop

1. **Extensión Estricta**: Cada página pública o privada del frontend debe heredar obligatoriamente de la estructura maestra `base.html` mediante la sentencia `{% extends "base.html" %}`.
2. **Inyección Tipada**: Está prohibido pasar hashes sueltos u objetos anónimos a los templates de Askama; encapsula la información en structs derivados de `Template` en los controladores Axum.
3. **Control de Navbar (`current_user`)**: Todos los structs de templates que utilicen el Navbar maestro deben inyectar el campo `current_user: Option<UserSessionDto>` para conmutar dinámicamente los botones de sesión.
4. **Auto-Cierre de Alertas Flash**: Implementa el script JS maestro en `base.html` para desvanecer automáticamente las notificaciones flash (`flash-alert`) tras 4 segundos de exposición visual.
5. **No Exponer SQL**: Bajo ninguna circunstancia muestres mensajes de error técnicos directos (como fallos de base de datos) en las vistas de usuario; delega en la plantilla de error unificada.
6. **Mapeo Responsive de Tarjeta**: Todas las rejillas de catálogos deben renderizarse usando el componente **`listing-card.html`** integrado bajo un contenedor responsive mobile-first de Tailwind (`grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6`).
7. **Favoritos Asíncronos**: La interacción para guardar favoritos debe interceptarse en JavaScript mediante `fetch()`, impidiendo recargas completas de pantalla y alterando los estilos visuales de la tarjeta localmente en caliente.
8. **Estandarización de Avatares**: Utiliza siempre el componente de avatar adaptativo (`user-avatar`) para garantizar que los perfiles que carezcan de imagen Cloudinary muestren un marcador visual tipográfico limpio.
9. **camelCase en DTOs de Plantilla**: Asegura que cualquier DTO de datos serializado hacia JavaScript para interacciones asíncronas cuente con la anotación `#[serde(rename_all = "camelCase")]`.
10. **Paginación Uniforme**: Las vistas de catálogos y resultados que implementen listados de registros deben adjuntar obligatoriamente el componente de paginación estructurado a partir del struct `PageResult` de `crates/common`.
