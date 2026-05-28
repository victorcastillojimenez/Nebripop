use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Entidad que representa un anuncio guardado como favorito.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorite {
    pub id: Uuid,
    pub user_id: Uuid,
    pub listing_id: Uuid,
    pub created_at: DateTime<Utc>,
}
