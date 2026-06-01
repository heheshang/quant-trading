//! Signed OKX Client — HMAC-SHA256 签名客户端
//!
//! 参考 SignedBinanceClient 架构实现
//! OKX 签名算法: signStr = timestamp + method + request_path + body, then base64(HMAC-SHA256(secret, signStr))
//! Headers: OK-ACCESS-KEY, OK-ACCESS-SIGN, OK-ACCESS-TIMESTAMP (ms), OK-ACCESS-PASSPHRASE

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use std::sync::Arc;
use uuid::Uuid;

use crate::services::exchange::api_keys::ApiKeyStore;
use crate::services::exchange::rate_limiter::RateLimiter;
use crate::utils::error::AppError;

type HmacSha256 = Hmac<sha2::Sha256>;

const OKX_API: &str = "https://www.okx.com";

// ─── Response Types ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PingResponse {
    pub server_time: i64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Balance {
    pub asset: String,
    pub available: String,
    pub frozen: String,
}

#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub balances: Vec<Balance>,
}

#[derive(Debug, Clone)]
pub struct Fill {
    pub price: String,
    pub qty: String,
    pub fee: String,
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

#[derive(Debug, Clone)]
pub struct PendingOrder {
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub price: String,
    pub qty: String,
    pub status: String,
}

// ─── NewOrder ───────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct NewOrder {
    pub inst_id: String,
    pub td_mode: String,
    pub side: String,
    pub ord_type: String,
    pub sz: String,
    pub px: Option<String>,
}

// ─── SignedOkxClient ────────────────────────────────────────

pub struct SignedOkxClient {
    key_store: Arc<ApiKeyStore>,
    rate_limiter: RateLimiter,
    http_client: reqwest::Client,
}

impl Clone for SignedOkxClient {
    fn clone(&self) -> Self {
        Self {
            key_store: self.key_store.clone(),
            rate_limiter: RateLimiter::new(), // rate limiter per-instance (stateless)
            http_client: reqwest::Client::new(),
        }
    }
}

impl SignedOkxClient {
    pub fn new(key_store: Arc<ApiKeyStore>) -> Self {
        Self {
            key_store,
            rate_limiter: RateLimiter::new(),
            http_client: reqwest::Client::new(),
        }
    }

    /// 生成 OKX 签名
    fn sign(timestamp: &str, method: &str, request_path: &str, body: &str, secret: &str) -> String {
        let sign_str = format!("{}{}{}{}", timestamp, method, request_path, body);
        let mac = HmacSha256::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        let mut mac = mac;
        mac.update(sign_str.as_bytes());
        let result = mac.finalize();
        BASE64.encode(result.into_bytes())
    }

    /// 发送带签名的请求
    async fn signed_request(
        &self,
        method: &str,
        endpoint: &str,
        body: String,
        user_id: Uuid,
    ) -> Result<String, AppError> {
        // 1. Rate limit check
        self.rate_limiter.check_and_record(user_id).await?;

        // 2. Get API key + decrypt secret
        let api_key = self.key_store.get_api_key_plain(user_id, "okx").await?;
        let secret_encrypted = self
            .key_store
            .get_encrypted_for_decrypt(user_id, "okx")
            .await?;

        let parts: Vec<&str> = secret_encrypted.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(AppError::Internal("Invalid secret format".into()));
        }
        let (nonce_b64, combined_b64) = (parts[0], parts[1]);
        let secret = self
            .key_store
            .decrypt_secret_for_client(nonce_b64, combined_b64)?;

        // OKX passphrase is the secret itself for API trading
        let passphrase = &secret;

        // 3. Generate timestamp and sign
        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
        let signature = Self::sign(&timestamp, method, endpoint, &body, &secret);

        // 4. Send request
        let url = format!("{}{}", OKX_API, endpoint);

        let mut req_builder = match method {
            "GET" => self.http_client.get(&url),
            "POST" => self.http_client.post(&url),
            "DELETE" => self.http_client.delete(&url),
            _ => return Err(AppError::Internal(format!("Unsupported method: {}", method))),
        };

        req_builder = req_builder
            .header("OK-ACCESS-KEY", &api_key)
            .header("OK-ACCESS-SIGN", &signature)
            .header("OK-ACCESS-TIMESTAMP", &timestamp)
            .header("OK-ACCESS-PASSPHRASE", passphrase)
            .header("Content-Type", "application/json");

        if !body.is_empty() && method != "GET" {
            req_builder = req_builder.body(body);
        }

        let res = req_builder.send().await
            .map_err(|e| AppError::Internal(format!("HTTP error: {}", e)))?;

        let body = res.text().await
            .map_err(|e| AppError::Internal(format!("Body read error: {}", e)))?;

        // Update last used
        let _ = self.key_store.update_last_used(user_id, "okx").await;

        Ok(body)
    }

    /// GET /api/v5/account/balance → AccountInfo
    pub async fn get_account(&self, user_id: Uuid) -> Result<AccountInfo, AppError> {
        let body = self.signed_request("GET", "/api/v5/account/balance", String::new(), user_id).await?;

        #[derive(serde::Deserialize)]
        struct OkxAccount {
            data: Vec<OkxBalanceData>,
        }
        #[derive(serde::Deserialize)]
        struct OkxBalanceData {
            details: Vec<OkxBalanceDetail>,
        }
        #[derive(serde::Deserialize)]
        struct OkxBalanceDetail {
            #[serde(alias = "ccy")]
            ccy: String,
            #[serde(alias = "availBal")]
            avail_bal: Option<String>,
            #[serde(alias = "frozenBal")]
            frozen_bal: Option<String>,
        }

        let account: OkxAccount = serde_json::from_str(&body)
            .map_err(|e| AppError::Internal(format!("Parse account error: {}", e)))?;

        let balances: Vec<Balance> = account
            .data
            .into_iter()
            .flat_map(|d| d.details)
            .map(|detail| Balance {
                asset: detail.ccy,
                available: detail.avail_bal.unwrap_or_else(|| "0".to_string()),
                frozen: detail.frozen_bal.unwrap_or_else(|| "0".to_string()),
            })
            .collect();

        Ok(AccountInfo { balances })
    }

    /// POST /api/v5/trade/order → OrderResponse
    pub async fn place_order(
        &self,
        order: NewOrder,
        user_id: Uuid,
    ) -> Result<OrderResponse, AppError> {
        self.rate_limiter.check_and_record(user_id).await?;

        let body = serde_json::to_string(&order)
            .map_err(|e| AppError::Internal(format!("Serialize order error: {}", e)))?;

        let response = self.signed_request("POST", "/api/v5/trade/order", body, user_id).await?;

        #[derive(serde::Deserialize)]
        struct OkxOrderResponse {
            data: Vec<OkxOrderData>,
            msg: String,
        }
        #[derive(serde::Deserialize)]
        struct OkxOrderData {
            #[serde(alias = "ordId")]
            ord_id: String,
            #[serde(alias = "instId")]
            inst_id: String,
            #[serde(alias = "side")]
            side: String,
            #[serde(alias = "ordType")]
            ord_type: String,
            #[serde(alias = "state")]
            state: String,
            // `sz` is the order size in OKX's response envelope. Captured for
            // future use (post-fill reconciliation, partial-fill tracking).
            // Currently the upstream `signed_request` returns the size via
            // `fill_sz`; this field is kept for completeness.
            #[serde(alias = "sz")]
            #[allow(dead_code)]
            sz: String,
            #[serde(alias = "fillSz")]
            fill_sz: Option<String>,
            #[serde(alias = "fills")]
            fills: Option<Vec<OkxFill>>,
        }
        #[derive(serde::Deserialize)]
        struct OkxFill {
            #[serde(alias = "px")]
            px: String,
            #[serde(alias = "sz")]
            sz: String,
            #[serde(alias = "fee")]
            fee: String,
        }

        let order_resp: OkxOrderResponse = serde_json::from_str(&response)
            .map_err(|e| AppError::Internal(format!("Parse order response error: {}", e)))?;

        let data = order_resp.data.into_iter().next()
            .ok_or_else(|| AppError::Internal(format!("Empty order response: {}", order_resp.msg)))?;

        let fills: Vec<Fill> = data.fills
            .unwrap_or_default()
            .into_iter()
            .map(|f| Fill {
                price: f.px,
                qty: f.sz,
                fee: f.fee,
            })
            .collect();

        Ok(OrderResponse {
            order_id: data.ord_id,
            symbol: data.inst_id,
            side: data.side,
            order_type: data.ord_type,
            status: data.state,
            executed_qty: data.fill_sz.unwrap_or_else(|| "0".to_string()),
            fills,
        })
    }

    /// POST /api/v5/trade/cancel-order → CancelOrderResponse
    pub async fn cancel_order(
        &self,
        order_id: &str,
        symbol: &str,
        user_id: Uuid,
    ) -> Result<CancelOrderResponse, AppError> {
        self.rate_limiter.check_and_record(user_id).await?;

        #[derive(serde::Serialize)]
        struct CancelRequest {
            #[serde(rename = "instId")]
            inst_id: String,
            #[serde(rename = "ordId")]
            ord_id: String,
        }

        let cancel_req = CancelRequest {
            inst_id: symbol.to_string(),
            ord_id: order_id.to_string(),
        };

        let body = serde_json::to_string(&cancel_req)
            .map_err(|e| AppError::Internal(format!("Serialize cancel request error: {}", e)))?;

        let response = self.signed_request("POST", "/api/v5/trade/cancel-order", body, user_id).await?;

        #[derive(serde::Deserialize)]
        struct OkxCancelResponse {
            data: Vec<OkxCancelData>,
            msg: String,
        }
        #[derive(serde::Deserialize)]
        struct OkxCancelData {
            #[serde(alias = "ordId")]
            ord_id: String,
            #[serde(alias = "state")]
            state: String,
        }

        let cancel_resp: OkxCancelResponse = serde_json::from_str(&response)
            .map_err(|e| AppError::Internal(format!("Parse cancel response error: {}", e)))?;

        let data = cancel_resp.data.into_iter().next()
            .ok_or_else(|| AppError::Internal(format!("Empty cancel response: {}", cancel_resp.msg)))?;

        Ok(CancelOrderResponse {
            order_id: data.ord_id,
            status: data.state,
        })
    }

    /// GET /api/v5/trade/orders-pending?instType=SPOT&limit=20 → Vec<PendingOrder>
    pub async fn get_pending_orders(&self, user_id: Uuid) -> Result<Vec<PendingOrder>, AppError> {
        let endpoint = "/api/v5/trade/orders-pending?instType=SPOT&limit=20";
        let body = self.signed_request("GET", endpoint, String::new(), user_id).await?;

        #[derive(serde::Deserialize)]
        struct OkxPendingResponse {
            data: Vec<OkxPendingOrder>,
        }
        #[derive(serde::Deserialize)]
        struct OkxPendingOrder {
            #[serde(alias = "ordId")]
            ord_id: String,
            #[serde(alias = "instId")]
            inst_id: String,
            #[serde(alias = "side")]
            side: String,
            #[serde(alias = "ordType")]
            ord_type: String,
            #[serde(alias = "px")]
            px: String,
            #[serde(alias = "sz")]
            sz: String,
            #[serde(alias = "state")]
            state: String,
        }

        let pending_resp: OkxPendingResponse = serde_json::from_str(&body)
            .map_err(|e| AppError::Internal(format!("Parse pending orders error: {}", e)))?;

        let orders: Vec<PendingOrder> = pending_resp
            .data
            .into_iter()
            .map(|o| PendingOrder {
                order_id: o.ord_id,
                symbol: o.inst_id,
                side: o.side,
                order_type: o.ord_type,
                price: o.px,
                qty: o.sz,
                status: o.state,
            })
            .collect();

        Ok(orders)
    }

    /// GET /api/v5/market/ticker?instId=BTC-USDT → PingResponse (public, no signature)
    pub async fn ping(&self) -> Result<PingResponse, AppError> {
        let url = format!("{}/api/v5/market/ticker?instId=BTC-USDT", OKX_API);

        let res = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HTTP error: {}", e)))?;

        let body = res.text().await
            .map_err(|e| AppError::Internal(format!("Body read error: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct OkxPing {
            data: Vec<OkxPingData>,
        }
        #[derive(serde::Deserialize)]
        struct OkxPingData {
            #[serde(alias = "ts")]
            ts: Option<String>,
        }

        let ping: OkxPing = serde_json::from_str(&body)
            .map_err(|e| AppError::Internal(format!("Parse ping error: {}", e)))?;

        let server_time = ping.data.first()
            .and_then(|d| d.ts.as_ref())
            .and_then(|ts| ts.parse().ok())
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        Ok(PingResponse {
            server_time,
            status: "ok".to_string(),
        })
    }
}
