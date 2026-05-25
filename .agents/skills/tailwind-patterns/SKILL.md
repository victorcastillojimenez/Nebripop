---
name: tailwind-patterns
description: Directrices de diseño, sistema de diseño visual, y patrones de clases de Tailwind CSS para maquetar el frontend premium de Nebripop. Utiliza esta skill siempre que vayas a escribir, maquetar o modificar plantillas HTML, componentes de interfaz de usuario o estilos CSS en el proyecto.
---

# Tailwind CSS Design Patterns — Nebripop UI

Esta skill define el sistema de diseño visual, la paleta de colores oficial (inspirada en la identidad fresca y moderna de Wallapop), los estándares responsive y las clases utilitarias de **Tailwind CSS** para maquetar el frontend de **Nebripop**. El objetivo es garantizar que cada vista sea visualmente impactante, premium, responsiva (mobile-first) y coherente, utilizando la integración directa de Tailwind CSS vía CDN sin herramientas de compilación externas.

---

## 1. Sistema de Colores de Nebripop

Para reflejar la identidad visual de un marketplace moderno de segunda mano (Wallapop), utilizamos una paleta centrada en un verde turquesa vibrante como primario, combinado con tonos complementarios para estados de interacción y componentes Stripe.

### Configuración del Config de Tailwind (Insertar en `base.html`)
```javascript
tailwind.config = {
    theme: {
        extend: {
            colors: {
                brand: {
                    50: '#f0fdfa',   // Fondo turquesa ultra-claro
                    100: '#ccfbf1',  // Fondos de badges
                    500: '#13c1ac',  // Verde Turquesa oficial (Wallapop)
                    600: '#0f9f8f',  // Estado Hover / Activo
                    700: '#0d7c70',  // Estado Focus / Texto oscuro
                },
                accent: {
                    50: '#faf5ff',
                    500: '#a855f7',  // Morado para favoritos o destacados
                    600: '#9333ea',
                },
                stripe: {
                    500: '#635bff',  // Color corporativo de Stripe para pagos
                    600: '#564fe8',
                }
            },
            fontFamily: {
                sans: ['Inter', 'sans-serif'],
                display: ['Outfit', 'sans-serif'],
            }
        }
    }
}
```

### Roles de Color en la UI
* **Primario (`brand`)**: Botones principales, enlaces activos, badges de precio y elementos de marca.
* **Neutros (`slate` o `gray`)**: Fondos de página (`slate-50`), bordes divisorios (`slate-100`/`slate-200`) y jerarquía de textos (`slate-800` para títulos, `slate-600` para cuerpo, `slate-400` para metadatos).
* **Estados / Feedback**:
  * **Éxito / Activo**: `emerald-500` (anuncios en estado activo).
  * **Error / Alerta**: `red-500` (acciones destructivas, corazones de favoritos, alertas de formularios).
  * **Valoración / Reputación**: `amber-400` (estrellas de ratings).

---

## 2. Pautas Responsive Mobile-First

El maquetado debe priorizar la visualización en dispositivos móviles (pantalla de 375px) y escalarse de forma fluida hacia tablets y ordenadores mediante prefijos responsive de Tailwind.

* **Vista Móvil (Por defecto)**: Disposición en una columna, anchos del 100%, márgenes reducidos (`p-4`), navbars colapsables u horizontales simplificados.
* **Vista Tablet (`md:` a partir de 768px)**: Disposición de dos columnas, elementos flotantes, menús más amplios.
* **Vista Desktop (`lg:` o `xl:` a partir de 1024px/1280px)**: Estructuras multipanel, filtros laterales permanentes, rejillas de 3 o 4 columnas.

### El Grid de Anuncios Estándar
Cualquier rejilla de catálogo de productos debe comportarse de forma responsiva de la siguiente manera:
```html
<div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6">
    <!-- Card Anuncio -->
</div>
```

---

## 3. Guía de Uso de Tailwind CSS via CDN (Sin Build Step)

Nebripop se ejecuta mediante plantillas Askama renderizadas del lado del servidor, por lo que **no existe un proceso de compilación (build step) de CSS**. Usamos la librería CDN oficial cargada en el `<head>`.

### Restricciones Críticas: Qué clases NO usar
1. **No usar directivas `@apply`**: Al no compilar CSS, no puedes crear clases personalizadas en un archivo CSS usando `@apply`. Utiliza clases utilitarias directamente en el HTML.
2. **Evitar Plugins no importados**: Clases que requieran plugins específicos (como `@tailwindcss/forms` o `@tailwindcss/line-clamp`) no funcionarán a menos que se cargue explícitamente su script adicional en la configuración del CDN.
3. **No usar clases arbitrarias dinámicas compiladas en Rust**: SQLx u Askama no deben concatenar strings de clases al vuelo (ej: `bg-[{{ color_variable }}]`). Tailwind via CDN escanea el DOM en tiempo de ejecución, pero es más seguro usar clases estáticas combinadas con lógica condicional en la plantilla:
   `class="p-2 {% if active %} bg-brand-500 {% else %} bg-slate-200 {% endif %}"`

---

## 4. Componente Completo: Navbar Premium Responsive

Navbar unificado e interactivo con estética *Glassmorphism*, adaptado a resoluciones móviles y estados condicionales (usuario logueado vs. anónimo).

```html
<!-- Componente: Navbar (base.html) -->
<header class="sticky top-0 z-50 w-full bg-white/80 backdrop-blur-md border-b border-slate-100 transition-all duration-300">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex h-16 items-center justify-between gap-4">
            
            <!-- Logo de Nebripop -->
            <div class="flex items-center gap-2">
                <a href="/" class="font-display font-bold text-2xl text-brand-500 flex items-center gap-1.5 transition-transform hover:scale-[1.02]">
                    <span class="text-3xl">🎈</span>
                    <span class="tracking-tight text-slate-800">Nebri<span class="text-brand-500">pop</span></span>
                </a>
            </div>

            <!-- Buscador Integrado en Desktop -->
            <div class="hidden md:flex flex-grow max-w-lg">
                <form action="/search" method="GET" class="w-full relative">
                    <input type="text" 
                           name="q" 
                           placeholder="Buscar en todas las categorías..." 
                           class="w-full h-10 pl-4 pr-10 rounded-full border border-slate-200 focus:border-brand-500 focus:ring-2 focus:ring-brand-100 outline-none text-sm transition-all bg-slate-50 focus:bg-white text-slate-700">
                    <button type="submit" class="absolute right-3 top-2.5 text-slate-400 hover:text-brand-500 transition-colors">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
                    </button>
                </form>
            </div>

            <!-- Navegación y Acciones -->
            <nav class="flex items-center gap-4">
                <!-- Enlace rápido de búsqueda para móviles -->
                <a href="/search" class="md:hidden p-2 text-slate-500 hover:text-brand-500 transition-colors" aria-label="Buscar">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
                </a>

                {% if current_user.is_some() %}
                    {% let user = current_user.as_ref().unwrap() %}
                    <!-- Botón Subir Producto -->
                    <a href="/listings/create" class="hidden sm:flex items-center gap-1.5 bg-brand-500 hover:bg-brand-600 text-white px-4 py-2 rounded-2xl font-semibold shadow-md shadow-brand-500/10 hover:shadow-lg hover:shadow-brand-500/20 -translate-y-[1px] hover:-translate-y-[2px] active:translate-y-0 transition-all duration-200 text-sm">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path></svg>
                        <span>Subir anuncio</span>
                    </a>

                    <!-- Mensajes / Chats -->
                    <a href="/chat" class="relative p-2 text-slate-500 hover:text-brand-500 hover:bg-slate-50 rounded-xl transition-all" aria-label="Mensajes">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path></svg>
                        <!-- Badge indicador de mensajes no leídos (ejemplo dinámico) -->
                        <span class="absolute top-1.5 right-1.5 w-2.5 h-2.5 bg-brand-500 rounded-full ring-2 ring-white"></span>
                    </a>

                    <!-- Perfil Dropdown / Enlace -->
                    <a href="/users/me" class="flex items-center gap-2 group">
                        <div class="w-8 h-8 rounded-full bg-brand-100 text-brand-600 font-bold flex items-center justify-center border border-brand-200 transition-colors group-hover:bg-brand-500 group-hover:text-white">
                            {{ user.display_name.chars().next().unwrap() }}
                        </div>
                        <span class="hidden md:inline text-sm font-semibold text-slate-700 group-hover:text-brand-500 transition-colors">{{ user.display_name }}</span>
                    </a>
                {% else %}
                    <!-- Botones para estado anónimo -->
                    <a href="/login" class="text-sm font-semibold text-slate-600 hover:text-brand-500 transition-colors px-3 py-2 rounded-xl">Iniciar Sesión</a>
                    <a href="/register" class="text-sm font-semibold bg-brand-500 hover:bg-brand-600 text-white px-4 py-2 rounded-2xl shadow-sm transition-all">Registrarse</a>
                {% endif %}
            </div>
        </div>
    </div>
</header>
```

---

## 5. Componente Completo: Card de Anuncio (Listing Card)

Maquetación de la tarjeta del producto, optimizada con hover states avanzados, sombras elegantes, badges de estado y geolocalización.

```html
<!-- Componente: Card de Anuncio (listings/list.html) -->
<div class="group relative bg-white rounded-3xl overflow-hidden border border-slate-100 shadow-sm hover:shadow-xl hover:-translate-y-1.5 transition-all duration-300 flex flex-col h-full">
    
    <!-- Imagen del Anuncio -->
    <div class="relative aspect-[4/3] w-full overflow-hidden bg-slate-50">
        <img src="{{ item.image_url }}" 
             alt="{{ item.title }}" 
             class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
             loading="lazy">
        
        <!-- Badge de precio flotante superior izquierdo -->
        <div class="absolute top-3 left-3 bg-white/95 backdrop-blur-sm px-3.5 py-1 rounded-2xl shadow-md border border-slate-100 flex items-center justify-center">
            <span class="font-display font-bold text-slate-800 text-base md:text-lg">{{ item.price }}</span>
        </div>

        <!-- Botón de Favorito flotante superior derecho (Interacción con JS) -->
        <button class="absolute top-3 right-3 w-8 h-8 rounded-full bg-white/90 backdrop-blur-sm hover:bg-white flex items-center justify-center text-slate-400 hover:text-red-500 shadow-md border border-slate-100 transition-all active:scale-90 btn-favorite" 
                data-listing-id="{{ item.id }}"
                aria-label="Añadir a favoritos">
            <svg class="w-4 h-4 fill-current" viewBox="0 0 24 24"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/></svg>
        </button>

        <!-- Badge de estado físico inferior izquierdo -->
        <div class="absolute bottom-3 left-3 flex gap-1.5">
            {% if item.condition == "new" %}
                <span class="bg-emerald-500/90 backdrop-blur-sm text-white text-[10px] font-bold tracking-wider uppercase px-2 py-0.5 rounded-md shadow-sm">Nuevo</span>
            {% else if item.condition == "like_new" %}
                <span class="bg-brand-500/90 backdrop-blur-sm text-white text-[10px] font-bold tracking-wider uppercase px-2 py-0.5 rounded-md shadow-sm">Como Nuevo</span>
            {% else %}
                <span class="bg-amber-500/90 backdrop-blur-sm text-white text-[10px] font-bold tracking-wider uppercase px-2 py-0.5 rounded-md shadow-sm">Usado</span>
            {% endif %}
        </div>
    </div>

    <!-- Información del Anuncio -->
    <div class="p-4 flex-grow flex flex-col justify-between gap-3 bg-white">
        <div class="space-y-1">
            <!-- Título -->
            <h3 class="font-display font-semibold text-slate-800 text-sm md:text-base line-clamp-1 group-hover:text-brand-500 transition-colors">
                {{ item.title }}
            </h3>
            
            <!-- Ubicación y Distancia -->
            <p class="text-xs text-slate-400 flex items-center gap-1">
                <svg class="w-3.5 h-3.5 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>
                <span>{{ item.city }}</span>
                {% if item.distance_km.is_some() %}
                    <span class="text-slate-300">•</span>
                    <span class="text-brand-600 font-medium">a {{ item.distance_km.unwrap() }} km</span>
                {% endif %}
            </p>
        </div>

        <!-- Botón de acción -->
        <a href="/listings/{{ item.id }}" 
           class="block w-full text-center bg-slate-50 hover:bg-brand-50 text-slate-600 hover:text-brand-600 py-2.5 rounded-2xl font-semibold text-xs md:text-sm transition-all duration-200">
            Ver detalles del producto
        </a>
    </div>
</div>
```

---

## 6. Formularios y Estados de Interacción (Hover/Focus/Error)

Todos los formularios en Nebripop deben tener un estilo unificado y fluido, mostrando respuestas de foco inmediatas y colores de error controlados.

```html
<!-- Componente: Formulario de Registro / Creación -->
<div class="space-y-4 max-w-md mx-auto bg-white p-6 rounded-3xl border border-slate-100 shadow-sm">
    
    <!-- Input Estándar -->
    <div class="space-y-1.5">
        <label for="reg-email" class="block text-xs font-bold uppercase tracking-wider text-slate-500">Correo Electrónico</label>
        <input type="email" 
               id="reg-email" 
               placeholder="ejemplo@nebrija.es"
               class="w-full h-11 px-4 rounded-xl border border-slate-200 focus:border-brand-500 focus:ring-4 focus:ring-brand-100 outline-none text-sm transition-all bg-slate-50 focus:bg-white text-slate-700">
    </div>

    <!-- Input con Error de Validación (Controlado por clases) -->
    <div class="space-y-1.5">
        <label for="reg-password" class="block text-xs font-bold uppercase tracking-wider text-slate-500">Contraseña</label>
        <!-- border-red-500 y ring-red-100 para estados incorrectos -->
        <input type="password" 
               id="reg-password" 
               value="123"
               class="w-full h-11 px-4 rounded-xl border border-red-500 focus:border-red-500 focus:ring-4 focus:ring-red-100 outline-none text-sm transition-all bg-white text-slate-700">
        <p class="text-xs text-red-500 font-medium">La contraseña debe tener al menos 8 caracteres.</p>
    </div>

    <!-- Select Estándar -->
    <div class="space-y-1.5">
        <label for="reg-condition" class="block text-xs font-bold uppercase tracking-wider text-slate-500">Estado del Producto</label>
        <select id="reg-condition" 
                class="w-full h-11 px-4 rounded-xl border border-slate-200 focus:border-brand-500 focus:ring-4 focus:ring-brand-100 outline-none text-sm transition-all bg-slate-50 focus:bg-white text-slate-700">
            <option value="new">Nuevo</option>
            <option value="like_new">Como Nuevo</option>
            <option value="used">Usado</option>
        </select>
    </div>
</div>
```

---

## 7. Componentes de Interacción Avanzados

### A. Burbujas de Chat (`chat/room.html`)
El diseño del chat debe diferenciar claramente al remitente del destinatario usando colores complementarios y bordes curvos dinámicos.

```html
<!-- Burbuja Remitente (Comprador/Yo) -->
<div class="flex justify-end gap-2">
    <div class="bg-brand-500 text-white px-4 py-2.5 rounded-2xl rounded-tr-none max-w-xs md:max-w-md shadow-sm">
        <p class="text-sm leading-relaxed">Hola, ¿sigue disponible? Me interesa bastante el precio.</p>
        <span class="block text-[10px] text-brand-100 text-right mt-1">20:15</span>
    </div>
</div>

<!-- Burbuja Receptor (Vendedor) -->
<div class="flex justify-start gap-2">
    <div class="bg-white text-slate-700 px-4 py-2.5 rounded-2xl rounded-tl-none max-w-xs md:max-w-md shadow-sm border border-slate-100">
        <p class="text-sm leading-relaxed">¡Hola! Sí, sigue disponible. Hago envíos o entrega en mano en Madrid.</p>
        <span class="block text-[10px] text-slate-400 mt-1">20:17</span>
    </div>
</div>
```

### B. Modal de Pago Stripe (`payments/checkout.html`)
El modal para procesar pagos seguros con Stripe debe transmitir seguridad mediante colores neutros fuertes combinados con el morado Stripe.

```html
<!-- Backdrop del Modal -->
<div class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-sm flex items-center justify-center p-4">
    <!-- Contenedor del Modal -->
    <div class="w-full max-w-md bg-white rounded-3xl overflow-hidden shadow-2xl border border-slate-100 transform transition-all p-6 space-y-6">
        
        <!-- Cabecera -->
        <div class="text-center space-y-2">
            <span class="text-4xl">🔒</span>
            <h3 class="font-display font-bold text-xl text-slate-800">Pago Seguro de Nebripop</h3>
            <p class="text-xs text-slate-400">Tus datos financieros están 100% protegidos por Stripe.</p>
        </div>

        <!-- Detalle de Compra -->
        <div class="bg-slate-50 p-4 rounded-2xl border border-slate-100 flex justify-between items-center">
            <div>
                <h4 class="font-semibold text-sm text-slate-800">Bicicleta de montaña BH</h4>
                <p class="text-xs text-slate-400">Vendido por Victor C.</p>
            </div>
            <span class="font-display font-bold text-slate-800">150.00 €</span>
        </div>

        <!-- Botón Procesar Pago Stripe -->
        <button class="w-full bg-stripe-500 hover:bg-stripe-600 text-white py-3 rounded-2xl font-semibold shadow-lg shadow-stripe-500/10 hover:shadow-xl hover:shadow-stripe-500/20 transition-all flex items-center justify-center gap-2 text-sm">
            <span>Pagar con</span>
            <span class="font-display font-bold tracking-tight text-base">stripe</span>
        </button>
    </div>
</div>
```

---

## 8. Las 10 Reglas Críticas de UI/Tailwind para Nebripop

1. **Configuración Centralizada de Colores**: Utiliza siempre los colores oficiales del proyecto (`brand-500` para turquesa Wallapop, `accent-500` para morado y `stripe-500` para pasarelas) inyectados en la navbar y botones interactivos.
2. **Diseño Mobile-First Obligatorio**: Diseña todas las pantallas pensando primero en resoluciones móviles de 375px. Escala hacia layouts multipanel de tablet (`md:`) y escritorio (`lg:`) mediante prefijos responsivos.
3. **Card Estandarizada de Producto**: Todas las rejillas de catálogos y búsquedas deben renderizar anuncios utilizando exactamente la estructura de la **Card de Anuncio** (con badges de estado, badge de precio flotante y botón de favoritos).
4. **Consistencia en Inputs y Selects**: Todos los elementos de formulario del proyecto deben compartir el mismo estilo visual (alto `h-11`, bordes redondeados `rounded-xl`, fondo `bg-slate-50` y anillos de foco `focus:ring-brand-100`).
5. **No Usar @apply**: Debido a la integración directa de Tailwind via CDN sin compilación previa, está prohibido escribir reglas de CSS tradicional que contengan `@apply`. Coloca las clases utilitarias directamente en el código HTML de las plantillas.
6. **Estados de Interacción Enriquecidos**: Cada botón, enlace o elemento clicable del DOM debe contar con respuestas táctiles inmediatas y estados visuales explícitos para hover (`hover:bg-...`), focus (`focus:ring-...`) y active (`active:scale-...`).
7. **Uso Exclusivo de Fuentes de Google**: Aplica de manera transversal la fuente `font-display` (*Outfit*) en títulos principales y `font-sans` (*Inter*) en textos y descripciones para dar una estética tipográfica premium.
8. **Glassmorphism y Efectos de Elevación**: Emplea clases de desenfoque de fondo y sombras sutiles (`bg-white/80 backdrop-blur-md border border-slate-100 shadow-sm`) en cabeceras pegajosas (sticky navbars), barras de filtrado y modales.
9. **Independencia de Clases Arbitrarias**: Evita el uso de valores arbitrarios inline que dependan de plugins no cargados (por ejemplo, layouts complejos de cuadrícula) o clases de tamaños fijos no estándar (`w-[289px]`) que comprometan el diseño fluido y responsive.
10. **Compatibilidad en Carga Perezosa (Lazy)**: Las imágenes dentro de las cards de anuncios deben incluir el atributo `loading="lazy"` para acelerar la carga del primer pintado de pantalla (First Contentful Paint < 1.5s).
