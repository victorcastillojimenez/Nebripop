use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::adapters::conversation_repo::ConversationRepository;
use crate::adapters::message_repo::MessageRepository;
use crate::connections::ActiveConnections;
use crate::usecases::process_message_usecase;

/// Handle the full lifecycle of a WebSocket connection
/// Uses tokio::select! to manage concurrent send/receive tasks
/// Guarantees cleanup of active_connections on disconnect
pub async fn handle_socket(
    socket: WebSocket,
    conversation_repo: ConversationRepository,
    message_repo: MessageRepository,
    active_connections: ActiveConnections,
    user_id: Uuid,
    conversation_id: Uuid,
) {
    let (mut ws_sender, ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // 1. Register active connection
    active_connections
        .map
        .insert((conversation_id, user_id), tx);

    // 2. Send task: channel MPSC → WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 3. Receive task: WebSocket → process message
    let conv_repo_clone = conversation_repo.clone();
    let msg_repo_clone = message_repo.clone();
    let conn_clone = active_connections.clone();
    let mut receive_task = tokio::spawn(async move {
        let mut receiver = ws_receiver;
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Err(e) = process_message_usecase::process_received_text(
                    text,
                    user_id,
                    conversation_id,
                    &conv_repo_clone,
                    &msg_repo_clone,
                    &conn_clone,
                )
                .await
                {
                    tracing::error!(error = %e, "Error processing WS message");
                }
            }
        }
    });

    // 4. Wait for either task to finish (disconnection)
    tokio::select! {
        _ = (&mut send_task) => {}
        _ = (&mut receive_task) => {}
    }

    // 5. CLEANUP: Remove active connection
    send_task.abort();
    receive_task.abort();
    active_connections
        .map
        .remove(&(conversation_id, user_id));

    tracing::info!(
        "WebSocket disconnected: user {} conversation {}",
        user_id,
        conversation_id
    );
}
