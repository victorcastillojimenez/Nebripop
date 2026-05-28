use axum::extract::{Query, State};
use axum::Json;

use common::errors::AppError;
use common::pagination::PageRequest;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::auth::ChatUser;
use crate::dtos::ConversationListResponseDto;
use crate::usecases;

/// GET /chat — List conversations for the authenticated user
pub async fn handle(
    State(repo): State<ConversationRepository>,
    user: ChatUser,
    Query(pagination): Query<PageRequest>,
) -> Result<Json<ConversationListResponseDto>, AppError> {
    let result = usecases::list_conversations_usecase::execute(
        &repo,
        user.id,
        pagination.page,
        pagination.per_page,
    )
    .await
    .map_err(|e| {
        tracing::error!("Error listing conversations: {:?}", e);
        AppError::Internal(e.to_string())
    })?;

    Ok(Json(result))
}
