use sqlx::PgPool;

use chat::adapters::conversation_repo::ConversationRepository;
use chat::adapters::message_repo::MessageRepository;
use chat::connections::ActiveConnections;
use favorites::adapters::favorite_repository::FavoriteRepository;
use geo::adapters::geo_repository::GeoRepository;
use listings::adapters::cloudinary::ImageStorageImpl;
use listings::adapters::listing_repository::ListingRepositoryImpl;
use ratings::adapters::rating_repository::RatingRepository;
use search::adapters::meilisearch_adapter::MeiliSearchAdapter;
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
    pub listing_repo: ListingRepositoryImpl,
    pub image_storage: ImageStorageImpl,
    pub search_engine: Option<MeiliSearchAdapter>,
    pub active_connections: Arc<DashMap<(Uuid, Uuid), UnboundedSender<WsMessage>>>,
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
        let listing_repo = ListingRepositoryImpl::new(pool.clone());
        let image_storage = ImageStorageImpl::new();

        // Initialize MeiliSearch engine if configured
        let search_engine = Self::init_search_engine().await;

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
            listing_repo,
            image_storage,
            search_engine,
            active_connections: Arc::new(DashMap::new()),
        })
    }

    /// Initialize the MeiliSearch engine from environment variables.
    ///
    /// Requires `MEILISEARCH_URL` (default: `http://localhost:7700`).
    /// Optionally accepts `MEILISEARCH_API_KEY`.
    ///
    /// If `MEILISEARCH_URL` is not set, the engine is `None` (SQL fallback only).
    async fn init_search_engine() -> Option<MeiliSearchAdapter> {
        let meili_url = match std::env::var("MEILISEARCH_URL") {
            Ok(url) => url,
            Err(_) => {
                tracing::info!(
                    "MEILISEARCH_URL not set — search will use SQL ILIKE fallback"
                );
                return None;
            }
        };

        let api_key = std::env::var("MEILISEARCH_API_KEY").ok();

        let engine = MeiliSearchAdapter::new(&meili_url, api_key.as_deref());

        // Attempt index setup; warn on failure but don't crash startup
        match engine.setup_index().await {
            Ok(()) => {
                tracing::info!("MeiliSearch index configured successfully");
                Some(engine)
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to configure MeiliSearch index — search will use SQL ILIKE fallback"
                );
                None
            }
        }
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

impl axum::extract::FromRef<AppState> for ListingRepositoryImpl {
    fn from_ref(state: &AppState) -> Self {
        state.listing_repo.clone()
    }
}

impl axum::extract::FromRef<AppState> for ImageStorageImpl {
    fn from_ref(state: &AppState) -> Self {
        state.image_storage.clone()
    }
}

impl axum::extract::FromRef<AppState> for Option<MeiliSearchAdapter> {
    fn from_ref(state: &AppState) -> Self {
        state.search_engine.clone()
    }
}
