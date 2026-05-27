use axum::Json;

use crate::dtos::MessageResponse;

/// POST /auth/logout
/// Stateless logout — client discards the token
/// Returns 200 OK to confirm successful logout
pub async fn logout_handler() -> Json<MessageResponse> {
    Json(MessageResponse {
        message: "Sesión cerrada correctamente".to_string(),
    })
}
