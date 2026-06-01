use crate::models::schemas::WsQueryParams;
use crate::services::exchange::ws_hub::{HubMessage, WsHub};
use crate::utils::error::AppError;
use axum::{
    extract::{
        Extension, Query,
        ws::{Message, WebSocket, WebSocketUpgrade},
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
    #[serde(rename = "type")]
    msg_type: &'a str,
    symbol: &'a str,
    data: serde_json::Value,
}

/// Client subscription state (channel + symbol filters)
#[derive(Default)]
struct ClientSubscriptions {
    channels: HashSet<String>,
    symbols: HashSet<String>,
    /// Subscribe to personal trade execution notifications
    trade_subscribed: bool,
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
        HubMessage::Ticker {
            symbol,
            price,
            change,
            change_pct,
            volume,
            high,
            low,
            bid,
            ask,
        } => serde_json::to_string(&WsJsonMessage {
            msg_type: "ticker",
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
        .unwrap_or_default(),
        HubMessage::Depth {
            symbol,
            bids,
            asks,
            timestamp,
        } => serde_json::to_string(&WsJsonMessage {
            msg_type: "depth",
            symbol: &symbol,
            data: serde_json::json!({ "bids": bids, "asks": asks, "timestamp": timestamp }),
        })
        .unwrap_or_default(),
        HubMessage::Kline {
            symbol,
            interval,
            open,
            high,
            low,
            close,
            volume,
            timestamp,
        } => serde_json::to_string(&WsJsonMessage {
            msg_type: "kline",
            symbol: &symbol,
            data: serde_json::json!({
                "interval": interval,
                "open": open,
                "high": high,
                "low": low,
                "close": close,
                "volume": volume,
                "timestamp": timestamp,
            }),
        })
        .unwrap_or_default(),
        HubMessage::TradeExecuted {
            order_id,
            symbol,
            side,
            filled_quantity,
            avg_fill_price,
            is_fully_filled,
            realized_pnl,
            ..
        } => serde_json::to_string(&serde_json::json!({
            "type": "trade_executed",
            "symbol": symbol,
            "data": {
                "order_id": order_id.to_string(),
                "side": side,
                "filled_quantity": filled_quantity,
                "avg_fill_price": avg_fill_price,
                "is_fully_filled": is_fully_filled,
                "realized_pnl": realized_pnl,
            }
        }))
        .unwrap_or_default(),
        HubMessage::BacktestProgress {
            backtest_id,
            progress,
            status,
        } => serde_json::to_string(&WsJsonMessage {
            msg_type: "backtest_progress",
            symbol: &backtest_id.to_string(),
            data: serde_json::json!({
                "progress": progress,
                "status": status,
            }),
        })
        .unwrap_or_default(),
        HubMessage::AIPredict {
            symbol,
            interval,
            direction,
            confidence,
            signal,
            price_target,
            analysis,
            indicators,
            generated_at,
        } => serde_json::to_string(&WsJsonMessage {
            msg_type: "ai_predict",
            symbol: &symbol,
            data: serde_json::json!({
                "type": "prediction",
                "symbol": symbol,
                "interval": interval,
                "direction": direction,
                "confidence": confidence,
                "signal": signal,
                "price_target": price_target,
                "analysis": analysis,
                "indicators": indicators,
                "generated_at": generated_at,
            }),
        })
        .unwrap_or_default(),
    }
}

/// Check if a HubMessage matches client's subscription filters
fn message_matches_subscription(msg: &HubMessage, subs: &ClientSubscriptions) -> bool {
    // If no subscriptions configured, receive everything
    if subs.channels.is_empty() && subs.symbols.is_empty() {
        return true;
    }

    // TradeExecuted is gated by trade_subscribed flag
    if matches!(msg, HubMessage::TradeExecuted { .. }) {
        return subs.trade_subscribed;
    }

    // BacktestProgress is matched by channel subscription (backtest:progress:{id})
    if let HubMessage::BacktestProgress { backtest_id, .. } = &msg {
        let channel = format!("backtest:progress:{}", backtest_id);
        return subs.channels.is_empty() || subs.channels.contains(&channel);
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
        HubMessage::TradeExecuted { symbol, .. } => {
            let channel = "trade:executed".to_string();
            (channel, symbol.as_str())
        }
        HubMessage::BacktestProgress { .. } => {
            ("backtest:progress".to_string(), "")
        }
        HubMessage::AIPredict { symbol, .. } => {
            let channel = format!("ai:predict:{}", symbol);
            (channel, symbol.as_str())
        }
    };

    let channel_match = subs.channels.is_empty() || subs.channels.contains(&channel);
    let symbol_match = subs.symbols.is_empty() || subs.symbols.contains(symbol);

    channel_match && symbol_match
}

/// Map HubMessage type to a flat msg_type string for WS push
#[allow(dead_code)]
fn hub_msg_type(msg: &HubMessage) -> &'static str {
    match msg {
        HubMessage::Ticker { .. } => "ticker",
        HubMessage::Depth { .. } => "depth",
        HubMessage::Kline { .. } => "kline",
        HubMessage::TradeExecuted { .. } => "trade_executed",
        HubMessage::BacktestProgress { .. } => "backtest_progress",
        HubMessage::AIPredict { .. } => "ai_predict",
    }
}

/// GET /api/v1/ws — upgrade to WebSocket connection (JWT auth required)
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    Extension(ws_hub): Extension<Arc<WsHub>>,
) -> Result<impl IntoResponse, AppError> {
    // Validate JWT token from query param
    let claims = crate::services::auth::validate_token(&params.token, &crate::CONFIG.jwt_secret)?;
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::TokenInvalid("Invalid user token".to_string()))?;

    info!("WebSocket authenticated for user: {}", claims.username);

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, claims.jti, user_id, ws_hub)))
}

async fn handle_socket(
    socket: WebSocket,
    _session_id: String,
    user_id: uuid::Uuid,
    ws_hub: Arc<WsHub>,
) {
    info!("WebSocket connection established for user: {}", user_id);

    let (mut sender, mut receiver) = socket.split();
    let mut hub_rx = ws_hub.subscribe();
    let mut subscriptions = ClientSubscriptions {
        trade_subscribed: true,
        ..Default::default()
    };
    let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            // Send heartbeat to keep connection alive
            _ = heartbeat_interval.tick() => {
                let ts = chrono::Utc::now().timestamp();
                let heartbeat = format!(r#"{{"type":"heartbeat","ts":{}}}"#, ts);
                if sender.send(Message::Text(heartbeat.into())).await.is_err() {
                    break;
                }
            }
            // Forward market data from hub → WS client (filtered by subscription)
            msg = hub_rx.recv() => {
                match msg {
                    Ok(hub_msg) => {
                        // For TradeExecuted, route only to the target user
                        if matches!(&hub_msg, HubMessage::TradeExecuted { user_id: uid, .. } if *uid != user_id) {
                            continue;
                        }

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
                                    if let Some(symbol) = client_msg.symbol && !symbol.is_empty() {
                                        subscriptions.symbols.insert(symbol);
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
                                    let _ = sender.send(Message::Text(format!("echo: {}", text).into())).await;
                                }
                            }
                        } else {
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

    info!("WebSocket connection closed for user: {}", user_id);
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
