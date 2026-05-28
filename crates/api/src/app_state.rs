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
use users::adapters::user_repository::UserRepository;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
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

        let stripe_secret_key = std::env::var("STRIPE_SECRET_KEY")
            .map_err(|_| "STRIPE_SECRET_KEY must be set".to_string())?;

        let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
            .map_err(|_| "STRIPE_WEBHOOK_SECRET must be set".to_string())?;

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

        Ok(Self {
            pool,
            jwt_secret,
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
        })
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
