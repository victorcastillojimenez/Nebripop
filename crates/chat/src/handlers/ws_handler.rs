use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use common::errors::AppError;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::adapters::message_repo::MessageRepository;
use crate::connections::ActiveConnections;
use crate::usecases::ws_lifecycle_usecase;

/// JWT claims format
#[derive(Debug, Serialize, Deserialize)]
struct WsClaims {
    sub: Uuid,
    role: String,
    exp: i64,
    iat: i64,
}

/// Query parameters for WebSocket upgrade
#[derive(Debug, Deserialize)]
pub struct WsQueryParams {
    pub token: String,
}

/// WS /chat/:id/ws — WebSocket upgrade handler
/// Validates JWT token BEFORE upgrading the connection
pub async fn handle(
    State(conversation_repo): State<ConversationRepository>,
    State(message_repo): State<MessageRepository>,
    State(active_connections): State<ActiveConnections>,
    State(jwt_secret): State<String>,
    Path(conversation_id): Path<Uuid>,
    Query(params): Query<WsQueryParams>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    // 1. Validate JWT token BEFORE upgrade
    let claims = decode::<WsClaims>(
        &params.token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|err| {
        if let jsonwebtoken::errors::ErrorKind::ExpiredSignature = err.kind() {
            AppError::Unauthorized("El token de sesión ha expirado".to_string())
        } else {
            AppError::Unauthorized("Token de sesión inválido".to_string())
        }
    })?;

    let user_id = claims.claims.sub;

    // 2. Verify conversation membership
    let is_member = conversation_repo
        .is_member(conversation_id, user_id)
        .await
        .map_err(|_| AppError::Internal("Error verificando membresía".to_string()))?;

    if !is_member {
        return Err(AppError::Forbidden(
            "No tienes acceso a esta conversación".to_string(),
        ));
    }

    // 3. Accept the WebSocket upgrade
    Ok(ws.on_upgrade(move |socket| {
        ws_lifecycle_usecase::handle_socket(
            socket,
            conversation_repo,
            message_repo,
            active_connections,
            user_id,
            conversation_id,
        )
    }))
}
