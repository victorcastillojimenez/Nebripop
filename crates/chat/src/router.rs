use axum::routing::{get, post};
use axum::Router;

use crate::handlers;

/// Build the chat router with all chat endpoints
/// All routes require authentication via ChatUser extractor
/// State type S must provide:
/// - ConversationRepository (via FromRef)
/// - MessageRepository (via FromRef)
/// - ActiveConnections (via FromRef)
/// - PgPool (via FromRef)
/// - String (jwt_secret) (via FromRef)
pub fn chat_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    crate::adapters::conversation_repo::ConversationRepository: axum::extract::FromRef<S>,
    crate::adapters::message_repo::MessageRepository: axum::extract::FromRef<S>,
    crate::connections::ActiveConnections: axum::extract::FromRef<S>,
    sqlx::PgPool: axum::extract::FromRef<S>,
    String: axum::extract::FromRef<S>,
{
    Router::new()
        // GET /chat — list conversations
        .route("/chat", get(handlers::list_conversations::handle))
        // POST /chat — create conversation
        .route("/chat", post(handlers::create_conversation::handle))
        // GET /chat/:id/messages — get messages (HTTP polling fallback)
        .route("/chat/:id/messages", get(handlers::get_messages::handle))
        // POST /chat/:id/messages — send message via HTTP
        .route("/chat/:id/messages", post(handlers::send_message::handle))
        // WS /chat/:id/ws — WebSocket real-time messaging
        .route(
            "/chat/:id/ws",
            get(handlers::ws_handler::handle),
        )
}
