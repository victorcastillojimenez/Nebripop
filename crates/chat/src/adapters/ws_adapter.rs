use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::connections::ActiveConnections;
use crate::ports::{ConversationPort, MessagePort};
use crate::usecases::process_message_usecase;

/// Registration result containing both split socket parts and the MPSC channel.
struct SocketParts {
    ws_sender: SplitSink<WebSocket, Message>,
    ws_receiver: SplitStream<WebSocket>,
    sender: mpsc::UnboundedSender<Message>,
    receiver: mpsc::UnboundedReceiver<Message>,
}

/// Split the WebSocket and create the MPSC channel.
fn create_socket_parts(socket: WebSocket) -> SocketParts {
    let (ws_sender, ws_receiver) = socket.split();
    let (sender, receiver) = mpsc::unbounded_channel::<Message>();
    SocketParts {
        ws_sender,
        ws_receiver,
        sender,
        receiver,
    }
}

/// Register the connection and spawn both send and receive tasks.
/// Returns the sender channel (for registration) and both join handles.
fn spawn_tasks(
    parts: SocketParts,
    user_id: Uuid,
    conversation_id: Uuid,
    conversation_port: impl ConversationPort + Clone + 'static,
    message_port: impl MessagePort + Clone + 'static,
    broadcaster: ActiveConnections,
) -> (mpsc::UnboundedSender<Message>, JoinHandle<()>, JoinHandle<()>) {
    let send_task = tokio::spawn(async move {
        let mut sender = parts.ws_sender;
        let mut receiver = parts.receiver;
        while let Some(message) = receiver.recv().await {
            if sender.send(message).await.is_err() {
                break;
            }
        }
    });

    let receive_task = tokio::spawn(async move {
        let mut stream = parts.ws_receiver;
        while let Some(Ok(message)) = stream.next().await {
            if let Message::Text(text) = message {
                if let Err(error) = process_message_usecase::process_received_text(
                    text,
                    user_id,
                    conversation_id,
                    &conversation_port,
                    &message_port,
                    &broadcaster,
                )
                .await
                {
                    tracing::error!(error = %error, "Error processing WS message");
                }
            }
        }
    });

    (parts.sender, send_task, receive_task)
}

/// Clean up after disconnection: abort tasks, remove from map, log.
fn cleanup(
    broadcaster: &ActiveConnections,
    send_task: JoinHandle<()>,
    receive_task: JoinHandle<()>,
    conversation_id: Uuid,
    user_id: Uuid,
) {
    send_task.abort();
    receive_task.abort();
    broadcaster.map.remove(&(conversation_id, user_id));

    tracing::info!(
        "WebSocket disconnected: user {} conversation {}",
        user_id,
        conversation_id
    );
}

/// Handles the full lifecycle of a WebSocket connection.
pub async fn handle_socket(
    socket: WebSocket,
    conversation_port: impl ConversationPort + Clone + 'static,
    message_port: impl MessagePort + Clone + 'static,
    broadcaster: ActiveConnections,
    user_id: Uuid,
    conversation_id: Uuid,
) {
    let socket_parts = create_socket_parts(socket);
    let (sender, mut send_task, mut receive_task) = spawn_tasks(
        socket_parts,
        user_id,
        conversation_id,
        conversation_port,
        message_port,
        broadcaster.clone(),
    );

    broadcaster
        .map
        .insert((conversation_id, user_id), sender);

    tokio::select! {
        _ = (&mut send_task) => {}
        _ = (&mut receive_task) => {}
    }

    cleanup(&broadcaster, send_task, receive_task, conversation_id, user_id);
}
