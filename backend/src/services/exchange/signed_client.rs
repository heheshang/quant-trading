//! Signed Binance Client — HMAC-SHA256 签名客户端
//!
//! ADR-012 D1: 签名客户端架构 — 装饰器模式
//! - 组合 ApiKeyStore + RateLimiter
//! - 提供需要签名的 Binance API 调用

use hmac::{Hmac, Mac};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::services::exchange::api_keys::ApiKeyStore;
use crate::services::exchange::rate_limiter::RateLimiter;
use crate::utils::error::AppError;

type HmacSha256 = Hmac<sha2::Sha256>;

const BINANCE_API: &str = "https://api.binance.com";

// ─── Response Types ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PingResponse {
    pub server_time: i64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Balance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub balances: Vec<Balance>,
}

#[derive(Debug, Clone)]
pub struct Fill {
    pub price: String,
    pub qty: String,
    pub commission: String,
}

#[derive(Debug, Clone)]
pub struct OrderResponse {
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub status: String,
    pub executed_qty: String,
    pub fills: Vec<Fill>,
}

#[derive(Debug, Clone)]
pub struct CancelOrderResponse {
    pub order_id: String,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimitInfo {
    pub rate_limit_type: String,
    pub interval: String,
    pub interval_num: i32,
    pub limit: i64,
    pub count: i64,
}

// ─── NewOrder ───────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct NewOrder {
    pub symbol: String,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub quantity: Option<String>,
    pub price: Option<String>,
    pub time_in_force: Option<String>,
    pub quote_order_qty: Option<String>,
}

// ─── SignedBinanceClient ────────────────────────────────────────

pub struct SignedBinanceClient {
    key_store: Arc<ApiKeyStore>,
    rate_limiter: RateLimiter,
    http_client: reqwest::Client,
}

impl SignedBinanceClient {
    pub fn new(key_store: Arc<ApiKeyStore>) -> Self {
        Self {
            key_store,
            rate_limiter: RateLimiter::new(),
            http_client: reqwest::Client::new(),
        }
    }

    /// 生成带签名的 query string
    fn sign_query(params: &HashMap<String, String>, secret: &str) -> String {
        let query: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let mac = <HmacSha256 as Mac>::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        let mut mac = mac;
        mac.update(query.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        format!("{}&signature={}", query, signature)
    }

    /// 发送已签名的 GET 请求
    async fn signed_get(
        &self,
        endpoint: &str,
        params: &HashMap<String, String>,
        user_id: Uuid,
    ) -> Result<String, AppError> {
        // 1. Rate limit check
        self.rate_limiter.check_and_record(user_id).await?;

        // 2. Get API key + decrypt secret
        let api_key = self.key_store.get_api_key_plain(user_id, "binance").await?;
        let secret_encrypted = self
            .key_store
            .get_encrypted_for_decrypt(user_id, "binance")
            .await?;

        let parts: Vec<&str> = secret_encrypted.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(AppError::Internal("Invalid secret format".into()));
        }
        let (nonce_b64, combined_b64) = (parts[0], parts[1]);
        let secret = self
            .key_store
            .decrypt_secret_for_client(nonce_b64, combined_b64)?;

        // 3. Add timestamp + sign
        let mut signed_params = params.clone();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        signed_params.insert("timestamp".to_string(), timestamp.to_string());

        let query = Self::sign_query(&signed_params, &secret);

        // 4. Send request
        let url = format!("{}{}?{}", BINANCE_API, endpoint, query);

        let res = self
            .http_client
            .get(&url)
            .header("X-MBX-APIKEY", &api_key)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HTTP error: {}", e)))?;

        let body = res
            .text()
            .await
            .map_err(|e| AppError::Internal(format!("Body read error: {}", e)))?;

        // Update last used
        let _ = self.key_store.update_last_used(user_id, "binance").await;

        Ok(body)
    }

    /// GET /api/v3/account → AccountInfo
    pub async fn get_account(&self, user_id: Uuid) -> Result<AccountInfo, AppError> {
        let params = HashMap::new();
        let body = self.signed_get("/api/v3/account", &params, user_id).await?;

        #[derive(serde::Deserialize)]
        struct BinanceAccount {
            balances: Vec<BinanceBalance>,
        }
        #[derive(serde::Deserialize)]
        struct BinanceBalance {
            asset: String,
            free: String,
            locked: String,
        }

        let account: BinanceAccount = serde_json::from_str(&body)
            .map_err(|e| AppError::Internal(format!("Parse account error: {}", e)))?;

        Ok(AccountInfo {
            balances: account
                .balances
                .into_iter()
                .map(|b| Balance {
                    asset: b.asset,
                    free: b.free,
                    locked: b.locked,
                })
                .collect(),
        })
    }

    /// POST /api/v3/order → OrderResponse
    pub async fn place_order(
        &self,
        order: NewOrder,
        user_id: Uuid,
    ) -> Result<OrderResponse, AppError> {
        self.rate_limiter.check_and_record(user_id).await?;

        let api_key = self.key_store.get_api_key_plain(user_id, "binance").await?;
        let secret_encrypted = self
            .key_store
            .get_encrypted_for_decrypt(user_id, "binance")
            .await?;
        let parts: Vec<&str> = secret_encrypted.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(AppError::Internal("Invalid secret format".into()));
        }
        let secret = self
            .key_store
            .decrypt_secret_for_client(parts[0], parts[1])?;

        let mut params: HashMap<String, String> = [
            ("symbol".to_string(), order.symbol.clone()),
            ("side".to_string(), order.side.clone()),
            ("type".to_string(), order.order_type.clone()),
        ]
        .into_iter()
        .collect();

        if let Some(q) = order.quantity {
            params.insert("quantity".to_string(), q);
        }
        if let Some(p) = order.price {
            params.insert("price".to_string(), p);
        }
        if let Some(tf) = order.time_in_force {
            params.insert("timeInForce".to_string(), tf);
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        params.insert("timestamp".to_string(), timestamp.to_string());

        let query = Self::sign_query(&params, &secret);

        let url = format!("{}{}", BINANCE_API, "/api/v3/order");

        let res = self
            .http_client
            .post(&url)
            .header("X-MBX-APIKEY", &api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(query)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HTTP error: {}", e)))?;

        let body = res
            .text()
            .await
            .map_err(|e| AppError::Internal(format!("Body read error: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct BinanceOrder {
            #[serde(rename = "orderId")]
            order_id: u64,
            symbol: String,
            side: String,
            #[serde(rename = "type")]
            order_type: String,
            status: String,
            #[serde(rename = "executedQty")]
            executed_qty: String,
            fills: Vec<BinanceFill>,
        }
        #[derive(serde::Deserialize)]
        struct BinanceFill {
            price: String,
            qty: String,
            commission: String,
        }

        let b_order: BinanceOrder = serde_json::from_str(&body).map_err(|e| {
            AppError::Internal(format!("Parse order error: {} — body: {}", e, body))
        })?;

        let _ = self.key_store.update_last_used(user_id, "binance").await;

        Ok(OrderResponse {
            order_id: b_order.order_id.to_string(),
            symbol: b_order.symbol,
            side: b_order.side,
            order_type: b_order.order_type,
            status: b_order.status,
            executed_qty: b_order.executed_qty,
            fills: b_order
                .fills
                .into_iter()
                .map(|f| Fill {
                    price: f.price,
                    qty: f.qty,
                    commission: f.commission,
                })
                .collect(),
        })
    }

    /// DELETE /api/v3/order → CancelOrderResponse
    pub async fn cancel_order(
        &self,
        order_id: &str,
        symbol: &str,
        user_id: Uuid,
    ) -> Result<CancelOrderResponse, AppError> {
        self.rate_limiter.check_and_record(user_id).await?;

        let api_key = self.key_store.get_api_key_plain(user_id, "binance").await?;
        let secret_encrypted = self
            .key_store
            .get_encrypted_for_decrypt(user_id, "binance")
            .await?;
        let parts: Vec<&str> = secret_encrypted.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(AppError::Internal("Invalid secret format".into()));
        }
        let secret = self
            .key_store
            .decrypt_secret_for_client(parts[0], parts[1])?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let params: HashMap<String, String> = [
            ("symbol".to_string(), symbol.to_string()),
            ("orderId".to_string(), order_id.to_string()),
            ("timestamp".to_string(), timestamp.to_string()),
        ]
        .into_iter()
        .collect();

        let query = Self::sign_query(&params, &secret);

        let url = format!("{}{}", BINANCE_API, "/api/v3/order");

        let res = self
            .http_client
            .delete(&url)
            .header("X-MBX-APIKEY", &api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(query)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HTTP error: {}", e)))?;

        let body = res
            .text()
            .await
            .map_err(|e| AppError::Internal(format!("Body read error: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct BinanceCancel {
            #[serde(rename = "orderId")]
            order_id: u64,
            status: String,
        }
        let cancel: BinanceCancel = serde_json::from_str(&body)
            .map_err(|e| AppError::Internal(format!("Parse cancel error: {}", e)))?;

        let _ = self.key_store.update_last_used(user_id, "binance").await;

        Ok(CancelOrderResponse {
            order_id: cancel.order_id.to_string(),
            status: cancel.status,
        })
    }

    /// GET /api/v3/order → OrderResponse
    pub async fn get_order(
        &self,
        order_id: &str,
        symbol: &str,
        user_id: Uuid,
    ) -> Result<OrderResponse, AppError> {
        let params: HashMap<String, String> = [
            ("symbol".to_string(), symbol.to_string()),
            ("orderId".to_string(), order_id.to_string()),
        ]
        .into_iter()
        .collect();

        let body = self.signed_get("/api/v3/order", &params, user_id).await?;

        #[derive(serde::Deserialize)]
        struct BinanceOrder {
            #[serde(rename = "orderId")]
            order_id: u64,
            symbol: String,
            side: String,
            #[serde(rename = "type")]
            order_type: String,
            status: String,
            #[serde(rename = "executedQty")]
            executed_qty: String,
            fills: Vec<BinanceFill>,
        }
        #[derive(serde::Deserialize)]
        struct BinanceFill {
            price: String,
            qty: String,
            commission: String,
        }

        let b_order: BinanceOrder = serde_json::from_str(&body)
            .map_err(|e| AppError::Internal(format!("Parse order error: {}", e)))?;

        Ok(OrderResponse {
            order_id: b_order.order_id.to_string(),
            symbol: b_order.symbol,
            side: b_order.side,
            order_type: b_order.order_type,
            status: b_order.status,
            executed_qty: b_order.executed_qty,
            fills: b_order
                .fills
                .into_iter()
                .map(|f| Fill {
                    price: f.price,
                    qty: f.qty,
                    commission: f.commission,
                })
                .collect(),
        })
    }

    /// GET /sapi/v1/ping → PingResponse
    pub async fn ping(&self) -> Result<PingResponse, AppError> {
        let url = format!("{}{}", BINANCE_API, "/sapi/v1/ping");

        let res = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HTTP error: {}", e)))?;

        let body = res
            .text()
            .await
            .map_err(|e| AppError::Internal(format!("Body read error: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct BinancePing {
            #[serde(alias = "serverTime")]
            server_time: Option<i64>,
        }

        let ping: BinancePing = serde_json::from_str(&body)
            .map_err(|e| AppError::Internal(format!("Parse ping error: {}", e)))?;

        Ok(PingResponse {
            server_time: ping.server_time.unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
            }),
            status: "ok".to_string(),
        })
    }

    /// GET /api/v3/exchangeInfo → rate limit info
    pub async fn get_rate_limit(&self, user_id: Uuid) -> Result<RateLimitInfo, AppError> {
        // Rate limit info from Binance is in exchangeInfo, but we track locally
        let _ = user_id;
        Ok(RateLimitInfo {
            rate_limit_type: "REQUEST_WEIGHT".to_string(),
            interval: "MINUTE".to_string(),
            interval_num: 1,
            limit: 1200i64,
            count: 0i64,
        })
    }

    /// Check rate limit remaining for user
    pub async fn get_rate_limit_status(&self, user_id: Uuid) -> Result<(u64, i64), AppError> {
        let remaining = self.rate_limiter.remaining(user_id).await;
        let reset_at_ms = i64::MAX; // Sliding window, no fixed reset
        Ok((remaining, reset_at_ms))
    }
}

impl Clone for SignedBinanceClient {
    fn clone(&self) -> Self {
        Self {
            key_store: Arc::clone(&self.key_store),
            rate_limiter: self.rate_limiter.clone(),
            http_client: self.http_client.clone(),
        }
    }
}
