/// WebSocket message serializer
#[derive(Serialize)]
struct WsJsonMessage<'a> {
    channel: &'a str,
    symbol: &'a str,
    data: serde_json::Value,
}

/// Serialize HubMessage → JSON string for WS client
fn serialize_hub_message(msg: HubMessage) -> String {
    match msg {
        HubMessage::Ticker { symbol, price, change, change_pct, volume, high, low, bid, ask } => {
            serde_json::to_string(&WsJsonMessage {
                channel: "market:ticker",
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
            }).unwrap_or_default()
        }
        HubMessage::Depth { symbol, bids, asks } => {
            serde_json::to_string(&WsJsonMessage {
                channel: "market:depth",
                symbol: &symbol,
                data: serde_json::json!({ "bids": bids, "asks": asks }),
            }).unwrap_or_default()
        }
        HubMessage::Kline { symbol, interval, open, high, low, close, volume } => {
            serde_json::to_string(&WsJsonMessage {
                channel: "market:kline",
                symbol: &symbol,
                data: serde_json::json!({
                    "interval": interval,
                    "open": open,
                    "high": high,
                    "low": low,
                    "close": close,
                    "volume": volume,
                }),
            }).unwrap_or_default()
        }
    }
}

async fn handle_socket(socket: WebSocket, _session_id: String, ws_hub: Arc<WsHub>) {
    info!("WebSocket connection established");

    let (mut sender, mut receiver) = socket.split();
    let mut hub_rx = ws_hub.subscribe();

    loop {
        tokio::select! {
            // Forward market data from hub → WS client
            msg = hub_rx.recv() => {
                match msg {
                    Ok(hub_msg) => {
                        let text = serialize_hub_message(hub_msg);
                        if !text.is_empty() && sender.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            // Receive messages from WS client
            ws_msg = receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        info!("WS received from client: {}", text);
                        // Echo back for now; later: parse subscribe/unsubscribe commands
                        let _ = sender.send(Message::Text(format!("echo: {}", text).into())).await;
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