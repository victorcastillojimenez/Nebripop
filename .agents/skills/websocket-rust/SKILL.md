---
name: websocket-rust
description: Directrices de arquitectura, mejores prácticas y patrones de codificación para implementar la mensajería en tiempo real con WebSockets (axum::extract::ws) y persistencia en PostgreSQL para el chat de Nebripop. Utiliza esta skill siempre que vayas a escribir, modificar o auditar endpoints de WebSockets, sincronización de estados concurrentes o reconexiones en el cliente.
---

# WebSocket & Real-time Chat Best Practices — Nebripop

Esta skill define las directrices arquitectónicas, el diseño de red, los mecanismos de persistencia y la sincronización asíncrona para el módulo de chat en tiempo real (`chat`) de **Nebripop**. Se detalla la implementación de WebSockets en el servidor Rust mediante **Axum**, la gestión de canales en memoria sobre **Tokio**, la persistencia transaccional en **PostgreSQL** y los mecanismos de contingencia (polling HTTP) y reconexión automática en el navegador.

---

## 1. El Handler de WebSocket en Axum (Upgrade de Conexión)

Axum proporciona el extractor `WebSocketUpgrade` para negociar el protocolo HTTP y elevarlo a una conexión bidireccional WebSocket duradera.

### Autenticación durante el Handshake
Dado que las cabeceras HTTP personalizadas (como `Authorization`) no son accesibles nativamente mediante el constructor estándar de JavaScript `new WebSocket(url)`, en Nebripop extraeremos el token JWT y el ID de la conversación de manera segura a través de **parámetros de consulta (Query string)** durante el handshake.

### Implementación del Endpoint de WebSocket en Axum

```rust
use axum::{
    extract::{State, Query, ws::{WebSocketUpgrade, WebSocket, Message}},
    response::IntoResponse,
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
    pub conversation_id: uuid::Uuid,
}

pub async fn chat_ws_handler(
    State(state): State<AppState>,
    Query(params): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    // 1. Validar el token JWT de forma estricta antes de aceptar la elevación de protocolo
    let claims = jwt::decode_jwt(&params.token, &state.jwt_secret)
        .map_err(|_| AppError::Unauthorized("Token inválido o expirado".to_string()))?;

    // 2. Verificar que el usuario pertenece a la conversación solicitada
    let is_member = chat::usecases::verify_conversation_membership(
        params.conversation_id, 
        claims.sub, 
        &state.db
    ).await?;

    if !is_member {
        return Err(AppError::Forbidden("No tienes acceso a esta conversación".to_string()));
    }

    // 3. Aceptar la elevación de protocolo y pasar la conexión a la tarea asíncrona de Tokio
    let user_id = claims.sub;
    let conversation_id = params.conversation_id;
    
    Ok(ws.on_upgrade(move |socket| {
        handle_socket_lifecycle(socket, state, user_id, conversation_id)
    }))
}
```

---

## 2. Gestión de Conexiones Concurrentes en el Servidor

Para enrutar mensajes entre usuarios conectados concurrentemente a la misma conversación de Nebripop, mantendremos un mapa de hilos de mensajería activos (`ActiveConnections`) en memoria compartida, utilizando canales asíncronos `tokio::sync::mpsc`.

### Estructura en `AppState`
```rust
use dashmap::DashMap;
use tokio::sync::mpsc;
use std::sync::Arc;

pub type TxChannel = mpsc::UnboundedSender<Message>;

// Llave compuesta: (conversation_id, user_id)
#[derive(Clone)]
pub struct ActiveConnections {
    pub map: Arc<DashMap<(uuid::Uuid, uuid::Uuid), TxChannel>>,
}
```

### El Ciclo de Vida del Socket (`handle_socket_lifecycle`)

```rust
use futures_util::{StreamExt, SinkExt};

pub async fn handle_socket_lifecycle(
    socket: WebSocket,
    state: AppState,
    user_id: uuid::Uuid,
    conversation_id: uuid::Uuid,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 1. Crear un canal asíncrono MPSC para enviar mensajes hacia esta conexión WebSocket
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // 2. Registrar la conexión activa del usuario en esta conversación
    state.active_connections.map.insert((conversation_id, user_id), tx);

    // 3. Hilo asíncrono para reenviar al WebSocket del navegador cualquier mensaje recibido en el canal MPSC
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break; // Conexión cerrada
            }
        }
    });

    // 4. Hilo principal: leer del WebSocket del navegador (Cliente -> Servidor)
    let state_clone = state.clone();
    let mut receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Message::Text(text) = msg {
                if let Err(err) = process_received_text(text, user_id, conversation_id, &state_clone).await {
                    tracing::error!("Error al procesar mensaje recibido: {:?}", err);
                }
            }
        }
    });

    // 5. Esperar a que cualquiera de las tareas falle o finalice (desconexión)
    tokio::select! {
        _ = (&mut send_task) => {}
        _ = (&mut receive_task) => {}
    }

    // 6. LIMPIEZA: El socket se ha cerrado. Remover conexión activa de memoria para evitar fugas
    send_task.abort();
    receive_task.abort();
    state.active_connections.map.remove(&(conversation_id, user_id));
}
```

---

## 3. Estructura de Mensajes y Flujo de Persistencia

Los mensajes de texto transmitidos por el socket deben ser JSON serializables y persistirse inmediatamente en PostgreSQL antes de ser reenviados al destinatario para garantizar la consistencia si alguno de los dos se desconecta.

### DTOs del Chat

```rust
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize)]
pub struct ChatMessagePayload {
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChatMessageResponse {
    pub id: uuid::Uuid,
    pub conversation_id: uuid::Uuid,
    pub sender_id: uuid::Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}
```

### Flujo de Recepción y Reenvío Asíncrono

```rust
pub async fn process_received_text(
    text: String,
    sender_id: uuid::Uuid,
    conversation_id: uuid::Uuid,
    state: &AppState,
) -> Result<(), AppError> {
    // 1. Deserializar payload entrante
    let payload: ChatMessagePayload = serde_json::from_str(&text)
        .map_err(|_| AppError::BadRequest("Formato JSON de mensaje no válido".to_string()))?;

    if payload.content.trim().is_empty() {
        return Ok(());
    }

    // 2. Persistir el mensaje en PostgreSQL usando SQLx (Garantía Must-Have de persistencia)
    let message_id = uuid::Uuid::new_v4();
    let saved_msg = sqlx::query_as!(
        ChatMessageResponse,
        r#"
        INSERT INTO messages (id, conversation_id, sender_id, content, created_at, is_read)
        VALUES ($1, $2, $3, $4, now(), false)
        RETURNING id, conversation_id, sender_id, content, created_at
        "#,
        message_id,
        conversation_id,
        sender_id,
        payload.content
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::DatabaseError)?;

    // Actualizar también el timestamp `updated_at` de la conversación para ordenarla
    sqlx::query!(
        "UPDATE conversations SET updated_at = now() WHERE id = $1",
        conversation_id
    )
    .execute(&state.db)
    .await
    .ok();

    // 3. Identificar al otro participante de la conversación (Destinatario)
    let conversation = sqlx::query!(
        "SELECT buyer_id, seller_id FROM conversations WHERE id = $1",
        conversation_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::DatabaseError)?;

    let recipient_id = if conversation.buyer_id == sender_id {
        conversation.seller_id
    } else {
        conversation.buyer_id
    };

    // 4. Si el destinatario está conectado concurrentemente, enviarle el mensaje en tiempo real
    if let Some(recipient_tx) = state.active_connections.map.get(&(conversation_id, recipient_id)) {
        let serialized_response = serde_json::to_string(&saved_msg).unwrap();
        
        // Enviar mensaje de texto a través del canal MPSC del destinatario
        recipient_tx.send(Message::Text(serialized_response)).ok();
    }

    Ok(())
}
```

---

## 4. Fallback a Polling HTTP (`GET /chat/:id/messages`)

Para mitigar el riesgo de redes inestables o navegadores sin soporte WebSocket, se expone un endpoint REST tradicional. El cliente puede realizar peticiones secuenciales rápidas filtrando por la fecha del último mensaje conocido (`since`).

```rust
#[derive(Deserialize)]
pub struct PollingQuery {
    // Timestamp del último mensaje en cliente para traer solo lo nuevo
    pub since: Option<DateTime<Utc>>,
}

pub async fn get_messages_polling(
    State(state): State<AppState>,
    claims: Claims,
    Path(conversation_id): Path<uuid::Uuid>,
    Query(query): Query<PollingQuery>,
) -> Result<Json<Vec<ChatMessageResponse>>, AppError> {
    // 1. Validar propiedad / pertenencia
    let is_member = chat::usecases::verify_conversation_membership(conversation_id, claims.sub, &state.db).await?;
    if !is_member {
        return Err(AppError::Forbidden("No tienes acceso a esta conversación".to_string()));
    }

    // 2. Traer mensajes nuevos
    let messages = if let Some(last_date) = query.since {
        sqlx::query_as!(
            ChatMessageResponse,
            r#"
            SELECT id, conversation_id, sender_id, content, created_at
            FROM messages
            WHERE conversation_id = $1 AND created_at > $2
            ORDER BY created_at ASC
            "#,
            conversation_id,
            last_date
        )
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as!(
            ChatMessageResponse,
            r#"
            SELECT id, conversation_id, sender_id, content, created_at
            FROM messages
            WHERE conversation_id = $1
            ORDER BY created_at ASC
            LIMIT 50
            "#,
            conversation_id
        )
        .fetch_all(&state.db)
        .await
    }
    .map_err(AppError::DatabaseError)?;

    Ok(Json(messages))
}
```

---

## 5. JavaScript Vanilla: Reconexión Automática con Backoff Exponencial

Para garantizar una experiencia de usuario fluida, el código JS del cliente debe ser capaz de reconectarse automáticamente si el túnel de WebSocket se cae repentinamente.

```javascript
// templates/chat/room.html
class ChatWebSocketClient {
    constructor(conversationId, jwt) {
        this.conversationId = conversationId;
        this.jwt = jwt;
        this.socket = null;
        this.reconnectAttempts = 0;
        this.maxReconnectInterval = 30000; // Máximo 30 segundos
    }

    connect() {
        const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        const wsUrl = `${protocol}//${window.location.host}/api/v1/chat/ws?token=${this.jwt}&conversation_id=${this.conversationId}`;
        
        console.log("Conectando a WebSocket...");
        this.socket = new WebSocket(wsUrl);

        this.socket.onopen = () => {
            console.log("WebSocket Conectado con éxito.");
            this.reconnectAttempts = 0; // Resetear intentos
        };

        this.socket.onmessage = (event) => {
            const message = JSON.parse(event.data);
            appendMessageToUi(message.content, false);
        };

        this.socket.onclose = () => {
            console.log("Conexión perdida. Iniciando reconexión...");
            this.scheduleReconnect();
        };

        this.socket.onerror = (err) => {
            console.error("Error en el WebSocket:", err);
            this.socket.close(); // Fuerza el disparo de onclose
        };
    }

    scheduleReconnect() {
        // Backoff exponencial para no saturar al servidor de Nebripop
        const delay = Math.min(
            1000 * Math.pow(2, this.reconnectAttempts), 
            this.maxReconnectInterval
        );
        this.reconnectAttempts++;

        console.log(`Reintentando conexión en ${delay / 1000} segundos...`);
        setTimeout(() => {
            this.connect();
        }, delay);
    }

    send(content) {
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            this.socket.send(JSON.stringify({ content: content }));
            return true;
        }
        return false; // Retornar falso si está desconectado (el UI usará fallback)
    }
}
```

---

## 6. Patrones Correctos vs. Incorrectos

### A. Autenticación en la Conexión de WebSocket

❌ **Incorrecto (Aceptar cualquier conexión sin token JWT en el handshake y esperar a que el cliente lo mande en el primer mensaje de texto. Altamente inseguro, expuesto a spam de conexiones fantasma)**
```rust
// Aceptar la conexión directamente sin comprobación
pub async fn chat_ws_unsafe(
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| async {
        // ... Esperar un primer mensaje del socket para validar ...
        // ¡Vulnerable a ataques de denegación de servicio abriendo sockets infinitos!
    })
}
```

✅ **Correcto (Exigir token JWT como parámetro URL en la query del handshake y validar antes de conceder la elevación HTTP)**
```rust
pub async fn chat_ws_safe(
    State(state): State<AppState>,
    Query(params): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    // Aborta de inmediato con 401 Unauthorized si el token no es válido
    let claims = jwt::decode_jwt(&params.token, &state.jwt_secret)?;
    
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, claims.sub)))
}
```

---

### B. Ciclo de Vida del Socket y Fugas de Conexiones (`Fugas de Memoria`)

❌ **Incorrecto (Añadir la conexión MPSC al mapa pero no eliminarla en caso de desconexión abrupta. La memoria del servidor se saturará en pocas horas)**
```rust
pub async fn handle_socket_leaky(socket: WebSocket, state: AppState, user_id: uuid::Uuid) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();
    
    state.active_connections.map.insert(user_id, tx);
    
    // Bucle infinito leyendo mensajes...
    while let Some(Ok(msg)) = receiver.next().await {
        // ... procesar
    }
    // ¡ERROR: Si el bucle termina porque el socket se cierra, la conexión tx sigue en active_connections.map!
}
```

✅ **Correcto (Garantizar la remoción mediante control explícito de finalización o select!)**
```rust
pub async fn handle_socket_clean(socket: WebSocket, state: AppState, user_id: uuid::Uuid) {
    let (mut sender, receiver) = socket.split();
    let (tx, rx) = mpsc::unbounded_channel();
    
    state.active_connections.map.insert(user_id, tx);

    // Ejecutar tareas usando tokio::select! y remover al finalizar de forma garantizada
    // ...
    state.active_connections.map.remove(&user_id); // ¡Limpieza obligatoria!
}
```

---

## 7. Las 10 Reglas Críticas de WebSockets para Nebripop

1. **Handshake Autenticado Obligatorio**: Nunca eleves una conexión a WebSocket si el token JWT enviado en los parámetros de consulta (Query string) de la URL no es válido o está ausente.
2. **Validación de Pertenencia**: El servidor debe validar en la base de datos que el usuario solicitante realmente forma parte de la conversación (`buyer_id` o `seller_id`) antes de permitirle abrir el socket.
3. **Limpieza de Conexiones Activas**: Asegura la eliminación inmediata de los canales de retransmisión `tx` del mapa en memoria (`active_connections`) tras cualquier desconexión del cliente para evitar fugas de memoria críticas.
4. **Persistencia Previa a la Transmisión**: Al recibir un mensaje del cliente, persiste primero el registro en PostgreSQL (`INSERT INTO messages`). Si la persistencia falla, aborta y no retransmitas el mensaje al destinatario para evitar inconsistencias visuales.
5. **Formato JSON Estricto**: Todo mensaje de texto intercambiado a través del WebSocket debe estar estrictamente estructurado en formato JSON y mapeado contra los DTOs oficiales de Rust (`ChatMessageResponse`).
6. **Uso de Canales Seguros MPSC**: Para la comunicación entre hilos asíncronos en Tokio, utiliza canales MPSC (`tokio::sync::mpsc`) y desdobla las conexiones de envío y recepción para prevenir el bloqueo de hilos.
7. **Límite de Tamaño de Mensaje**: Limpia y valida el tamaño del texto recibido en los mensajes del chat (ej. máximo 5000 caracteres) para prevenir ataques DoS por inyección de payloads de texto masivos.
8. **Reconexión Exponencial en Cliente**: El script de JavaScript del cliente debe implementar un algoritmo de backoff exponencial en el navegador para no saturar al backend Axum con reintentos simultáneos durante una caída de red.
9. **Endpoint de Polling de Contingencia**: Proporciona el endpoint HTTP `GET /chat/:id/messages` con filtros de tiempo (`since`) como alternativa funcional por si el cliente experimenta bloqueos o proxies incompatibles con WebSockets.
10. **Actualización de Conversaciones**: Cada vez que se envíe un mensaje a través del WebSocket, actualiza de forma asíncrona la fecha `updated_at` de la tabla `conversations` para ordenar correctamente la bandeja de chats del usuario.
