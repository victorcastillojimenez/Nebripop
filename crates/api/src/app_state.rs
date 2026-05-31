use axum::extract::FromRef;
use sqlx::PgPool;

use chat::adapters::conversation_repo::ConversationRepository;
use chat::adapters::message_repo::MessageRepository;
use chat::connections::ActiveConnections;
use favorites::adapters::favorite_repository::FavoriteRepository;
use geo::adapters::geo_repository::GeoRepository;
use listings::adapters::cloudinary::ImageStorageImpl;
use listings::adapters::listing_repository::ListingRepositoryImpl;
use payments::adapters::listing_adapter::PaymentsListingAdapter;
use payments::adapters::payment_repository::PostgresPaymentRepository;
use payments::adapters::stripe_adapter::StripeAdapter;
use ratings::adapters::rating_repository::RatingRepository;
use search::adapters::meilisearch_adapter::MeiliSearchAdapter;
use users::adapters::user_repository::UserRepository;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub stripe_publishable_key: String,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub user_repo: UserRepository,
    pub conversation_repo: ConversationRepository,
    pub message_repo: MessageRepository,
    pub active_connections: ActiveConnections,
    pub rating_repo: RatingRepository,
    pub favorite_repo: FavoriteRepository,
    pub geo_repo: GeoRepository,
    pub listing_repo: ListingRepositoryImpl,
    pub image_storage: ImageStorageImpl,
    pub payment_repo: PostgresPaymentRepository,
    pub stripe_adapter: StripeAdapter,
    pub listing_service: PaymentsListingAdapter,
    pub search_engine: Option<MeiliSearchAdapter>,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL must be set".to_string())?;

        let pool = PgPool::connect(&database_url)
            .await
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        // Run database migrations on startup
        tracing::info!("Running database migrations...");
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .map_err(|e| format!("Database migration failed: {}", e))?;
        tracing::info!("Database migrations applied successfully");

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET must be set".to_string())?;

        let stripe_publishable_key = std::env::var("STRIPE_PUBLISHABLE_KEY")
            .map_err(|_| "STRIPE_PUBLISHABLE_KEY must be set".to_string())?;

        let stripe_secret_key = std::env::var("STRIPE_SECRET_KEY")
            .map_err(|_| "STRIPE_SECRET_KEY must be set".to_string())?;

        let stripe_webhook_secret = match std::env::var("STRIPE_WEBHOOK_SECRET") {
            Ok(val) if !val.is_empty() => val,
            _ => {
                tracing::warn!("STRIPE_WEBHOOK_SECRET no está configurada — el webhook de Stripe rechazará todas las notificaciones. Configúrala con un valor válido de Stripe CLI.");
                String::new()
            }
        };

        let user_repo = UserRepository::new(pool.clone());
        let conversation_repo = ConversationRepository::new(pool.clone());
        let message_repo = MessageRepository::new(pool.clone());
        let active_connections = ActiveConnections::new();
        let rating_repo = RatingRepository::new(pool.clone());
        let favorite_repo = FavoriteRepository::new(pool.clone());
        let geo_repo = GeoRepository::new(pool.clone());
        let listing_repo = ListingRepositoryImpl::new(pool.clone());
        let image_storage = ImageStorageImpl::new();
        let payment_repo = PostgresPaymentRepository::new(pool.clone());
        let stripe_adapter = StripeAdapter::new(
            stripe_secret_key.clone(),
            stripe_webhook_secret.clone(),
        );
        let listing_service = PaymentsListingAdapter::new(pool.clone());

        // Initialize MeiliSearch engine if configured
        let search_engine = Self::init_search_engine().await;

        Ok(Self {
            pool,
            jwt_secret,
            stripe_publishable_key,
            stripe_secret_key,
            stripe_webhook_secret,
            user_repo,
            conversation_repo,
            message_repo,
            active_connections,
            rating_repo,
            favorite_repo,
            geo_repo,
            listing_repo,
            image_storage,
            payment_repo,
            stripe_adapter,
            listing_service,
            search_engine,
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

        let engine = match MeiliSearchAdapter::new(&meili_url, api_key.as_deref()) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to create MeiliSearch client — search will use SQL ILIKE fallback"
                );
                return None;
            }
        };

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

// ---------------------------------------------------------------------------
// FromRef implementations for extracting components from AppState
// ---------------------------------------------------------------------------

impl FromRef<AppState> for UserRepository {
    fn from_ref(state: &AppState) -> Self {
        state.user_repo.clone()
    }
}

impl FromRef<AppState> for ConversationRepository {
    fn from_ref(state: &AppState) -> Self {
        state.conversation_repo.clone()
    }
}

impl FromRef<AppState> for MessageRepository {
    fn from_ref(state: &AppState) -> Self {
        state.message_repo.clone()
    }
}

impl FromRef<AppState> for ActiveConnections {
    fn from_ref(state: &AppState) -> Self {
        state.active_connections.clone()
    }
}

impl FromRef<AppState> for RatingRepository {
    fn from_ref(state: &AppState) -> Self {
        state.rating_repo.clone()
    }
}

impl FromRef<AppState> for FavoriteRepository {
    fn from_ref(state: &AppState) -> Self {
        state.favorite_repo.clone()
    }
}

impl FromRef<AppState> for GeoRepository {
    fn from_ref(state: &AppState) -> Self {
        state.geo_repo.clone()
    }
}

impl FromRef<AppState> for String {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_secret.clone()
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for ListingRepositoryImpl {
    fn from_ref(state: &AppState) -> Self {
        state.listing_repo.clone()
    }
}

impl FromRef<AppState> for ImageStorageImpl {
    fn from_ref(state: &AppState) -> Self {
        state.image_storage.clone()
    }
}

// --- Payment-related FromRef implementations ---

impl FromRef<AppState> for PostgresPaymentRepository {
    fn from_ref(state: &AppState) -> Self {
        state.payment_repo.clone()
    }
}

impl FromRef<AppState> for StripeAdapter {
    fn from_ref(state: &AppState) -> Self {
        state.stripe_adapter.clone()
    }
}

impl FromRef<AppState> for PaymentsListingAdapter {
    fn from_ref(state: &AppState) -> Self {
        state.listing_service.clone()
    }
}

impl FromRef<AppState> for Option<MeiliSearchAdapter> {
    fn from_ref(state: &AppState) -> Self {
        state.search_engine.clone()
    }
}
