use sqlx::PgPool;

use chat::adapters::conversation_repo::ConversationRepository;
use chat::adapters::message_repo::MessageRepository;
use chat::connections::ActiveConnections;
use favorites::adapters::favorite_repository::FavoriteRepository;
use geo::adapters::geo_repository::GeoRepository;
use ratings::adapters::rating_repository::RatingRepository;
use users::adapters::user_repository::UserRepository;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub user_repo: UserRepository,
    pub conversation_repo: ConversationRepository,
    pub message_repo: MessageRepository,
    pub active_connections: ActiveConnections,
    pub rating_repo: RatingRepository,
    pub favorite_repo: FavoriteRepository,
    pub geo_repo: GeoRepository,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL must be set".to_string())?;

        let pool = PgPool::connect(&database_url)
            .await
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET must be set".to_string())?;

        let user_repo = UserRepository::new(pool.clone());
        let conversation_repo = ConversationRepository::new(pool.clone());
        let message_repo = MessageRepository::new(pool.clone());
        let active_connections = ActiveConnections::new();
        let rating_repo = RatingRepository::new(pool.clone());
        let favorite_repo = FavoriteRepository::new(pool.clone());
        let geo_repo = GeoRepository::new(pool.clone());

        Ok(Self {
            pool,
            jwt_secret,
            user_repo,
            conversation_repo,
            message_repo,
            active_connections,
            rating_repo,
            favorite_repo,
            geo_repo,
        })
    }
}

/// FromRef implementations for extracting components from AppState

impl axum::extract::FromRef<AppState> for UserRepository {
    fn from_ref(state: &AppState) -> Self {
        state.user_repo.clone()
    }
}

impl axum::extract::FromRef<AppState> for ConversationRepository {
    fn from_ref(state: &AppState) -> Self {
        state.conversation_repo.clone()
    }
}

impl axum::extract::FromRef<AppState> for MessageRepository {
    fn from_ref(state: &AppState) -> Self {
        state.message_repo.clone()
    }
}

impl axum::extract::FromRef<AppState> for ActiveConnections {
    fn from_ref(state: &AppState) -> Self {
        state.active_connections.clone()
    }
}

impl axum::extract::FromRef<AppState> for RatingRepository {
    fn from_ref(state: &AppState) -> Self {
        state.rating_repo.clone()
    }
}

impl axum::extract::FromRef<AppState> for FavoriteRepository {
    fn from_ref(state: &AppState) -> Self {
        state.favorite_repo.clone()
    }
}

impl axum::extract::FromRef<AppState> for GeoRepository {
    fn from_ref(state: &AppState) -> Self {
        state.geo_repo.clone()
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
