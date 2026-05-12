use crate::utils::error::AppError;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::models::schemas::WsQueryParams;

/// WebSocket manager for broadcasting market data
#[derive(Clone)]
pub struct WsManager {
    pub tx: broadcast::Sender<String>,
}

impl WsManager {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }
}

impl Default for WsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/v1/ws — upgrade to WebSocket connection (JWT auth required)
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    // Validate JWT token from query param
    let claims = crate::services::auth::validate_token(&params.token, &crate::CONFIG.jwt_secret)?;

    info!("WebSocket authenticated for user: {}", claims.username);

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, claims.jti)))
}

async fn handle_socket(socket: WebSocket, _session_id: String) {
    info!("WebSocket connection established");

    // Simple echo for now — will expand to market data subscription
    let (mut sender, mut receiver) = socket.split();

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                info!("WS received: {}", text);
                if sender
                    .send(Message::Text(text.to_string().into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => continue,
            Err(e) => {
                warn!("WS error: {}", e);
                break;
            }
        }
    }

    info!("WebSocket connection closed");
}

/// Health check endpoint
pub async fn health_check() -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({
        "code": 0,
        "data": {
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        },
        "message": "success"
    }))
}
