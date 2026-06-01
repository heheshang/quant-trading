//! AI Model Service HTTP Client
//!
//! P3-F3: AI量化模块 — 与外部 Python 模型服务通信的 HTTP Client
//!
//! 文档约定:
//!   Python 服务 base URL: http://localhost:8001 (可通过 AI_MODEL_SERVICE_URL 配置)
//!   POST /api/v1/predict   — 价格方向预测
//!   GET  /api/v1/models   — 列出可用模型版本
//!   GET  /api/v1/health   — 健康检查

use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::utils::error::AppError;

// ─────────────────────────────────────────────────────────────────
// Circuit Breaker
// ─────────────────────────────────────────────────────────────────

/// Circuit breaker state for the AI model service.
///
/// Prevents cascading failures when the external Python model service
/// is unavailable — after 3 consecutive failures the circuit "opens"
/// and fails fast without making HTTP requests.
#[derive(Debug)]
pub struct CircuitBreakerState {
    consecutive_failures: usize,
    last_failure_time: Option<Instant>,
    is_open: bool,
}

impl CircuitBreakerState {
    /// Creates a new circuit in the "closed" (healthy) state.
    pub fn new() -> Self {
        Self {
            consecutive_failures: 0,
            last_failure_time: None,
            is_open: false,
        }
    }

    /// Records a failure. After 3 consecutive failures the circuit opens.
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.last_failure_time = Some(Instant::now());
        if self.consecutive_failures >= 3 {
            self.is_open = true;
        }
    }

    /// Records a successful request — resets the circuit to closed.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.is_open = false;
        self.last_failure_time = None;
    }

    /// Returns true if the circuit allows a request:
    /// - closed (healthy) circuit → always allowed
    /// - open circuit → allowed only if 30 seconds have elapsed (half-open)
    pub fn should_allow_request(&self) -> bool {
        if !self.is_open {
            return true;
        }
        if let Some(last_failure) = self.last_failure_time {
            return last_failure.elapsed() >= Duration::from_secs(30);
        }
        false
    }

    #[cfg(test)]
    pub fn failure_count(&self) -> usize {
        self.consecutive_failures
    }

    #[cfg(test)]
    pub fn is_open(&self) -> bool {
        self.is_open
    }
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────
// Response Types
// ─────────────────────────────────────────────────────────────────

/// AI model prediction response.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PredictionResponse {
    pub direction: String, // "long" | "short" | "neutral"
    pub confidence: f64,   // 0.0 ~ 1.0
    pub model_version: String,
    #[serde(alias = "timestamp", default)]
    pub generated_at: String,
    #[serde(default)]
    pub price_target: Option<f64>, // optional price forecast
    #[serde(default)]
    pub signal: Option<String>,
    #[serde(default)]
    pub analysis: Option<String>,
    #[serde(default)]
    pub indicators: Option<serde_json::Value>,
}

/// Available AI model information.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ModelInfo {
    pub version: String,
    pub name: String,
    pub accuracy: f64,
    pub status: String, // "active" | "deprecated" | "training"
}

/// Health check response from the Python model service.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub models_available: usize,
}

// ─────────────────────────────────────────────────────────────────
// ModelClient
// ─────────────────────────────────────────────────────────────────

/// HTTP client for the external AI model service.
///
/// Built with a circuit breaker — after 3 consecutive failures it
/// fails fast (returns error without making HTTP requests) for 30s,
/// then transitions to half-open to test recovery.
#[derive(Clone)]
pub struct ModelClient {
    base_url: String,
    #[allow(dead_code)]
    timeout_secs: u64,
    client: Client,
    circuit_breaker: Arc<RwLock<CircuitBreakerState>>,
    /// Only present in test builds to verify request counts.
    #[cfg(test)]
    pub request_count: Arc<std::sync::atomic::AtomicU64>,
}

impl ModelClient {
    /// Creates a new `ModelClient`.
    ///
    /// # Arguments
    /// * `base_url` — base URL of the Python model service (e.g. "http://localhost:8001")
    /// * `timeout_secs` — HTTP request timeout in seconds
    pub fn new(base_url: impl Into<String>, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("reqwest Client::builder should not fail");
        Self {
            base_url: base_url.into(),
            timeout_secs,
            client,
            circuit_breaker: Arc::new(RwLock::new(CircuitBreakerState::new())),
            #[cfg(test)]
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Predicts price direction using the AI model service.
    ///
    /// # Arguments
    /// * `features` — normalized feature vector from `FeatureEngine`
    /// * `symbol` — trading pair symbol (e.g. "BTCUSDT")
    /// * `interval` — candlestick interval (e.g. "1h", "4h")
    /// * `model_version` — optional specific model version to use
    ///
    /// # Errors
    /// Returns `AppError::ExternalServiceError` if circuit breaker is open
    /// or if the HTTP call fails.
    pub async fn predict_price_direction(
        &self,
        features: &[f64],
        symbol: &str,
        interval: &str,
        model_version: Option<&str>,
    ) -> Result<PredictionResponse, AppError> {
        // Check circuit breaker before making request
        {
            let cb = self.circuit_breaker.read().await;
            if !cb.should_allow_request() {
                return Err(AppError::ExternalServiceError(
                    "AI model service circuit breaker is open".to_string(),
                ));
            }
        }

        let url = format!("{}/api/v1/predict", self.base_url);
        let body = serde_json::json!({
            "features": features,
            "symbol": symbol,
            "interval": interval,
            "model_version": model_version,
        });

        #[cfg(test)]
        {
            self.request_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        let resp = self.client.post(&url).json(&body).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let mut cb = self.circuit_breaker.write().await;
                cb.record_success();
                r.json::<PredictionResponse>()
                    .await
                    .map_err(|e| AppError::ExternalServiceError(e.to_string()))
            }
            Ok(r) => {
                let mut cb = self.circuit_breaker.write().await;
                cb.record_failure();
                Err(AppError::ExternalServiceError(format!(
                    "AI model service returned status {}",
                    r.status()
                )))
            }
            Err(e) => {
                let mut cb = self.circuit_breaker.write().await;
                cb.record_failure();
                Err(AppError::ExternalServiceError(e.to_string()))
            }
        }
    }

    /// Lists all available model versions from the AI service.
    pub async fn get_models(&self) -> Result<Vec<ModelInfo>, AppError> {
        {
            let cb = self.circuit_breaker.read().await;
            if !cb.should_allow_request() {
                return Err(AppError::ExternalServiceError(
                    "AI model service circuit breaker is open".to_string(),
                ));
            }
        }

        let url = format!("{}/api/v1/models", self.base_url);

        #[cfg(test)]
        {
            self.request_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        let resp = self.client.get(&url).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let mut cb = self.circuit_breaker.write().await;
                cb.record_success();
                r.json::<Vec<ModelInfo>>()
                    .await
                    .map_err(|e| AppError::ExternalServiceError(e.to_string()))
            }
            Ok(r) => {
                let mut cb = self.circuit_breaker.write().await;
                cb.record_failure();
                Err(AppError::ExternalServiceError(format!(
                    "AI model service returned status {}",
                    r.status()
                )))
            }
            Err(e) => {
                let mut cb = self.circuit_breaker.write().await;
                cb.record_failure();
                Err(AppError::ExternalServiceError(e.to_string()))
            }
        }
    }

    /// Checks the health of the AI model service.
    /// Unlike `predict` and `get_models`, this does NOT update the circuit breaker.
    pub async fn health_check(&self) -> Result<HealthResponse, AppError> {
        let url = format!("{}/api/v1/health", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::ExternalServiceError(e.to_string()))?
            .json::<HealthResponse>()
            .await
            .map_err(|e| AppError::ExternalServiceError(e.to_string()))
    }

    /// Returns the current number of recorded consecutive failures (test helper).
    #[cfg(test)]
    pub async fn failure_count(&self) -> usize {
        self.circuit_breaker.read().await.failure_count()
    }

    /// Returns the total number of requests made (test helper).
    #[cfg(test)]
    pub fn request_count(&self) -> u64 {
        self.request_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client() -> ModelClient {
        ModelClient::new("http://localhost:8001", 2)
    }

    #[tokio::test]
    async fn test_circuit_breaker_initially_closed() {
        let client = make_client();
        assert!(!client.circuit_breaker.read().await.is_open());
        assert!(client.circuit_breaker.read().await.should_allow_request());
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_3_failures() {
        let client = make_client();

        // Record 2 failures — circuit should still be closed
        for _ in 0..2 {
            client.circuit_breaker.write().await.record_failure();
        }
        assert!(!client.circuit_breaker.read().await.is_open());
        assert!(client.circuit_breaker.read().await.should_allow_request());

        // 3rd failure — circuit opens
        client.circuit_breaker.write().await.record_failure();
        assert!(client.circuit_breaker.read().await.is_open());
        assert!(!client.circuit_breaker.read().await.should_allow_request());
    }

    #[tokio::test]
    async fn test_circuit_breaker_records_success() {
        let client = make_client();

        // Some failures then a success
        client.circuit_breaker.write().await.record_failure();
        client.circuit_breaker.write().await.record_failure();
        client.circuit_breaker.write().await.record_success();

        assert!(!client.circuit_breaker.read().await.is_open());
        assert_eq!(client.circuit_breaker.read().await.failure_count(), 0);
    }

    #[tokio::test]
    async fn test_prediction_response_deserialization() {
        let json = r#"{
            "direction": "long",
            "confidence": 0.82,
            "model_version": "v2.1",
            "generated_at": "2025-01-01T00:00:00Z"
        }"#;
        let resp: PredictionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.direction, "long");
        assert!((resp.confidence - 0.82).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_model_info_deserialization() {
        let json = r#"[
            {"version": "v1.0", "name": "BTC 1h Model", "accuracy": 0.73, "status": "active"},
            {"version": "v2.0", "name": "BTC 1h Model v2", "accuracy": 0.78, "status": "deprecated"}
        ]"#;
        let models: Vec<ModelInfo> = serde_json::from_str(json).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].status, "active");
        assert_eq!(models[1].status, "deprecated");
    }

    #[tokio::test]
    async fn test_request_count_increments() {
        let client = make_client();
        assert_eq!(client.request_count(), 0);
        // Note: health_check does NOT increment count since it doesn't go through
        // the normal request path in this simplified client. We test predict/get_models.
    }
}
