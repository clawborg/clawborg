use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::time;

use crate::types::AppState;

/// Server sends a ping every 30 s to keep the connection alive through
/// proxies and load balancers that close idle connections.
const PING_INTERVAL: Duration = Duration::from_secs(30);

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

    // Send file change events to the client, plus a periodic ping to keep
    // the connection alive. If the send fails (dead connection), the task ends.
    let send_task = tokio::spawn(async move {
        let mut ping_interval = time::interval(PING_INTERVAL);
        ping_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        ping_interval.tick().await; // skip the immediate first tick

        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            let json = serde_json::to_string(&event).unwrap_or_default();
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                _ = ping_interval.tick() => {
                    if sender.send(Message::Ping(Default::default())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Receive messages from the client. Pong frames confirm the connection is
    // alive. Close frames trigger a clean shutdown.
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Pong(_) => {} // heartbeat acknowledged, nothing to do
                _ => {}
            }
        }
    });

    // When either task ends, abort the other immediately.
    // Without this, a disconnected client leaves its send_task running
    // indefinitely, accumulating orphaned tasks and broadcast subscriptions.
    let send_abort = send_task.abort_handle();
    let recv_abort = recv_task.abort_handle();
    tokio::select! {
        _ = send_task => { recv_abort.abort(); },
        _ = recv_task => { send_abort.abort(); },
    }

    tracing::info!("WebSocket client disconnected");
}
