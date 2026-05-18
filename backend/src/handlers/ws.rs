use crate::models::schemas::WsQueryParams;
use crate::services::exchange::ws_hub::{HubMessage, WsHub};
use crate::utils::error::AppError;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Extension, Query,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{info, warn};

/// WebSocket message serializer
#[derive(Serialize)]
struct WsJsonMessage<'a> {
    channel: &'a str,
    symbol: &'a str,
    data: serde_json::Value,
}

/// Client subscription state (channel + symbol filters)
#[derive(Default)]
struct ClientSubscriptions {
    channels: HashSet<String>,
    symbols: HashSet<String>,
}

/// Subscribe/unsubscribe message from client
#[derive(Debug, Deserialize)]
struct WsClientMessage {
    action: String,
    channels: Option<Vec<String>>,
    symbol: Option<String>,
}

/// Serialize HubMessage → JSON string for WS client
fn serialize_hub_message(msg: HubMessage) -> String {
    match msg {
        HubMessage::Ticker { symbol, price, change, change_pct, volume, high, low, bid, ask } => {
            serde_json::to_string(&WsJsonMessage {
                channel: &format!("market:ticker:{}", symbol),
                symbol: &symbol,
                data: serde_json::json!({
                    "price": price,
                    "change": change,
                    "changePercent": change_pct,
                    "volume": volume,
                    "high": high,
                    "low": low,
                    "bid": bid,
                    "ask": ask,
                }),
            })
            .unwrap_or_default()
        }
        HubMessage::Depth { symbol, bids, asks } => serde_json::to_string(&WsJsonMessage {
            channel: &format!("market:depth:{}", symbol),
            symbol: &symbol,
            data: serde_json::json!({ "bids": bids, "asks": asks }),
        })
        .unwrap_or_default(),
        HubMessage::Kline { symbol, interval, open, high, low, close, volume } => {
            serde_json::to_string(&WsJsonMessage {
                channel: &format!("market:kline:{}", symbol),
                symbol: &symbol,
                data: serde_json::json!({
                    "interval": interval,
                    "open": open,
                    "high": high,
                    "low": low,
                    "close": close,
                    "volume": volume,
                }),
            })
            .unwrap_or_default()
        }
    }
}

/// Check if a HubMessage matches client's subscription filters
fn message_matches_subscription(msg: &HubMessage, subs: &ClientSubscriptions) -> bool {
    // If no subscriptions configured, receive everything
    if subs.channels.is_empty() && subs.symbols.is_empty() {
        return true;
    }

    let (channel, symbol) = match msg {
        HubMessage::Ticker { symbol, .. } => {
            let channel = format!("market:ticker:{}", symbol);
            (channel, symbol.as_str())
        }
        HubMessage::Depth { symbol, .. } => {
            let channel = format!("market:depth:{}", symbol);
            (channel, symbol.as_str())
        }
        HubMessage::Kline { symbol, .. } => {
            let channel = format!("market:kline:{}", symbol);
            (channel, symbol.as_str())
        }
    };

    let channel_match = subs.channels.is_empty() || subs.channels.contains(&channel);
    let symbol_match = subs.symbols.is_empty() || subs.symbols.contains(symbol);

    channel_match && symbol_match
}

/// GET /api/v1/ws — upgrade to WebSocket connection (JWT auth required)
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    Extension(ws_hub): Extension<Arc<WsHub>>,
) -> Result<impl IntoResponse, AppError> {
    // Validate JWT token from query param
    let claims = crate::services::auth::validate_token(&params.token, &crate::CONFIG.jwt_secret)?;

    info!("WebSocket authenticated for user: {}", claims.username);

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, claims.jti, ws_hub)))
}

async fn handle_socket(socket: WebSocket, _session_id: String, ws_hub: Arc<WsHub>) {
    info!("WebSocket connection established");

    let (mut sender, mut receiver) = socket.split();
    let mut hub_rx = ws_hub.subscribe();
    let mut subscriptions = ClientSubscriptions::default();

    loop {
        tokio::select! {
            // Forward market data from hub → WS client (filtered by subscription)
            msg = hub_rx.recv() => {
                match msg {
                    Ok(hub_msg) => {
                        if message_matches_subscription(&hub_msg, &subscriptions) {
                            let text = serialize_hub_message(hub_msg);
                            if !text.is_empty() && sender.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            // Receive messages from WS client (subscribe/unsubscribe)
            ws_msg = receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        info!("WS received from client: {}", text);
                        // Parse subscribe/unsubscribe messages
                        if let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&text) {
                            match client_msg.action.as_str() {
                                "subscribe" => {
                                    if let Some(channels) = client_msg.channels {
                                        for ch in channels {
                                            if !ch.is_empty() {
                                                subscriptions.channels.insert(ch);
                                            }
                                        }
                                    }
                                    if let Some(symbol) = client_msg.symbol {
                                        if !symbol.is_empty() {
                                            subscriptions.symbols.insert(symbol);
                                        }
                                    }
                                    info!("Client subscribed: channels={:?}, symbols={:?}",
                                        subscriptions.channels, subscriptions.symbols);
                                    let _ = sender.send(Message::Text(
                                        r#"{"action":"subscribed","status":"ok"}"#.into()
                                    )).await;
                                }
                                "unsubscribe" => {
                                    if let Some(channels) = client_msg.channels {
                                        for ch in channels {
                                            subscriptions.channels.remove(&ch);
                                        }
                                    }
                                    if let Some(symbol) = client_msg.symbol {
                                        subscriptions.symbols.remove(&symbol);
                                    }
                                    let _ = sender.send(Message::Text(
                                        r#"{"action":"unsubscribed","status":"ok"}"#.into()
                                    )).await;
                                }
                                _ => {
                                    // Echo unknown messages
                                    let _ = sender.send(Message::Text(format!("echo: {}", text).into())).await;
                                }
                            }
                        } else {
                            // Echo non-JSON messages
                            let _ = sender.send(Message::Text(format!("echo: {}", text).into())).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        warn!("WS error: {}", e);
                        break;
                    }
                    _ => continue,
                }
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
