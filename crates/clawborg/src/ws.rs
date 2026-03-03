use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};

use crate::types::AppState;

/// GET /ws — WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.file_events_tx.subscribe();

    tracing::info!("WebSocket client connected");

    // Send file change events to client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let json = serde_json::to_string(&event).unwrap_or_default();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Receive pings/pongs from client (keep alive)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            } // Ignore other client messages for now
        }
    });

    // Abort handles let us cancel the other task without consuming the JoinHandles
    // via select!. Without this, a disconnected client leaves its send_task running
    // indefinitely, accumulating orphaned tasks and broadcast subscriptions.
    let send_abort = send_task.abort_handle();
    let recv_abort = recv_task.abort_handle();
    tokio::select! {
        _ = send_task => { recv_abort.abort(); },
        _ = recv_task => { send_abort.abort(); },
    }

    tracing::info!("WebSocket client disconnected");
}
