use askama_axum::IntoResponse;
use askama::Template;
use axum::{extract::{State, Path}, response::Html};
use axum_extra::extract::CookieJar;
use crate::app_state::AppState;
use users::dtos::UserDto;
use chat::dtos::ConversationResponseDto;
use chat::ports::ConversationPort;
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
    
    let user_id = match current_user.as_ref() {
        Some(user) => user.id,
        None => return axum::response::Redirect::to("/login").into_response(),
    };

    let conversations = match chat::usecases::list_conversations_usecase::execute(
        &state.conversation_repo,
        user_id,
        1,
        100,
    )
    .await
    {
        Ok(res) => res.conversations,
        Err(e) => {
            tracing::error!("Error listing conversations: {}", e);
            vec![]
        }
    };

    let template = ChatListTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        conversations,
        query_param: None,
    };
    Html(template.render().unwrap()).into_response()
}

pub async fn conversation_handler(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let current_user = crate::web::get_current_user(auth, &state).await;
    
    let user_id = match current_user.as_ref() {
        Some(user) => user.id,
        None => return axum::response::Redirect::to("/login").into_response(),
    };

    let auth_token = jar
        .get("session_token")
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    // Verify membership and fetch messages
    let messages_dtos = match chat::usecases::get_messages_usecase::execute(
        &state.conversation_repo,
        &state.message_repo,
        id,
        user_id,
        None,
        100,
    )
    .await
    {
        Ok(msgs) => msgs,
        Err(e) => {
            tracing::error!("Error fetching messages: {}", e);
            return axum::response::Redirect::to("/chat").into_response();
        }
    };

    // Enrich with conversation metadata (other participant, listing info)
    let (conversations, _) = state
        .conversation_repo
        .find_by_user_id_paginated(user_id, 1, 100)
        .await
        .unwrap_or((vec![], 0));

    let conversation = conversations.into_iter().find(|c| c.id == id);

    let (other_user_name, other_user_avatar, listing_title, listing_id) = if let Some(c) = conversation {
        (c.other_user_name, c.other_user_avatar, c.listing_title, c.listing_id)
    } else {
        ("Usuario".to_string(), None, "Producto".to_string(), Uuid::nil())
    };

    let messages = messages_dtos
        .into_iter()
        .map(|msg| {
            let is_own = msg.sender_id == user_id;
            let sent_at = msg.created_at.format("%H:%M").to_string();
            MessageResponseWrapper {
                is_own,
                content: msg.content,
                sent_at,
            }
        })
        .collect();

    let template = ConversationTemplate {
        current_user,
        flash_success: None,
        flash_error: None,
        conversation_id: id,
        other_user_name,
        other_user_avatar,
        listing_title,
        listing_id,
        messages,
        auth_token,
        current_user_id: user_id,
        query_param: None,
    };
    Html(template.render().unwrap()).into_response()
}
