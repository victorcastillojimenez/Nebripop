use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::{State, Path}, response::Html};
use crate::app_state::AppState;
use users::dtos::UserDto;
use chat::dtos::{ConversationResponseDto};
use uuid::Uuid;
use crate::web::filters;
use common::auth::AuthUser;

#[derive(Template)]
#[template(path = "chat/list.html")]
pub struct ChatListTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub conversations: Vec<ConversationResponseDto>,
    pub query_param: Option<String>,
}

#[derive(Template)]
#[template(path = "chat/conversation.html")]
pub struct ConversationTemplate {
    pub current_user: Option<UserDto>,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
    pub conversation_id: Uuid,
    pub other_user_name: String,
    pub other_user_avatar: Option<String>,
    pub listing_title: String,
    pub listing_id: Uuid,
    pub messages: Vec<MessageResponseWrapper>,
    pub auth_token: String,
    pub current_user_id: Uuid,
    pub query_param: Option<String>,
}

pub struct MessageResponseWrapper {
    pub is_own: bool,
    pub content: String,
    pub sent_at: String,
}

pub async fn chat_list_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
) -> impl IntoResponse {
    let current_user = crate::web::get_current_user(auth, &state).await;
    let template = ChatListTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        conversations: vec![],
        query_param: None,
    };
    Html(template.render().unwrap())
}

pub async fn conversation_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    Path(_id): Path<Uuid>,
) -> impl IntoResponse {
    let current_user = crate::web::get_current_user(auth, &state).await;
    let current_user_id = current_user.as_ref().map(|u| u.id).unwrap_or_else(Uuid::new_v4);
    // Basic mock because we need it to compile and render base.html
    let template = ConversationTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        conversation_id: _id,
        other_user_name: "Usuario".to_string(),
        other_user_avatar: None,
        listing_title: "Producto".to_string(),
        listing_id: Uuid::new_v4(),
        messages: vec![],
        auth_token: "".to_string(),
        current_user_id,
        query_param: None,
    };
    Html(template.render().unwrap())
}
