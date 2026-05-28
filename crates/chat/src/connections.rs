use axum::extract::ws::Message;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

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
