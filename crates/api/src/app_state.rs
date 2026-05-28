use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use users::adapters::user_repository::UserRepository;

/// Message type for WebSocket connections (placeholder for chat feature)
#[derive(Debug, Clone)]
pub struct WsMessage {
    pub text: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub user_repo: UserRepository,
    pub active_connections: Arc<DashMap<(Uuid, Uuid), UnboundedSender<WsMessage>>>,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL must be set".to_string())?;

        let pool = PgPool::connect(&database_url).await
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET must be set".to_string())?;

        let user_repo = UserRepository::new(pool.clone());

        Ok(Self {
            pool,
            jwt_secret,
            user_repo,
            active_connections: Arc::new(DashMap::new()),
        })
    }
}

/// Extractors using FromRef pattern
impl axum::extract::FromRef<AppState> for UserRepository {
    fn from_ref(state: &AppState) -> Self {
        state.user_repo.clone()
    }
}

impl axum::extract::FromRef<AppState> for String {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_secret.clone()
    }
}

impl axum::extract::FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}
