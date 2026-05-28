use async_trait::async_trait;
use axum::extract::ws::Message;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::ports::BroadcastPort;

/// Channel sender type for WebSocket messages
pub type TxChannel = mpsc::UnboundedSender<Message>;

/// Active WebSocket connections indexed by (conversation_id, user_id)
/// Clone is cheap because the DashMap is behind Arc
#[derive(Clone)]
pub struct ActiveConnections {
    pub map: Arc<DashMap<(Uuid, Uuid), TxChannel>>,
}

impl ActiveConnections {
    pub fn new() -> Self {
        Self {
            map: Arc::new(DashMap::new()),
        }
    }
}

impl Default for ActiveConnections {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BroadcastPort for ActiveConnections {
    /// Send a JSON payload to a specific user in a conversation via WebSocket.
    /// Silently ignores if the user is not connected.
    async fn send_to_user(
        &self,
        conversation_id: Uuid,
        recipient_id: Uuid,
        json_payload: &str,
    ) {
        if let Some(sender) = self.map.get(&(conversation_id, recipient_id)) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_payload) {
                let ws_message = Message::Text(serde_json::to_string(&json).unwrap_or_default());
                if sender.send(ws_message).is_err() {
                    tracing::warn!(
                        "Failed to send WS message to user {} in conversation {}",
                        recipient_id,
                        conversation_id
                    );
                }
            }
        }
    }
}
