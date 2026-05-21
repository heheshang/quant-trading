use reqwest::Client;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct HttpStressClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HttpResult {
    pub latency_ms: u64,
    pub status: u16,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StressTestResults {
    pub results: Vec<HttpResult>,
    pub total_requests: u64,
    pub total_duration_ms: u64,
}

impl Default for StressTestResults {
    fn default() -> Self {
        Self::new()
    }
}

impl StressTestResults {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            total_requests: 0,
            total_duration_ms: 0,
        }
    }

    pub fn p50_ms(&self) -> u64 {
        self.percentile(0.5)
    }

    pub fn p99_ms(&self) -> u64 {
        self.percentile(0.99)
    }

    fn percentile(&self, p: f64) -> u64 {
        if self.results.is_empty() {
            return 0;
        }
        let mut latencies: Vec<u64> = self.results.iter().map(|r| r.latency_ms).collect();
        latencies.sort();
        let idx = ((latencies.len() as f64) * p).ceil() as usize;
        latencies[idx.saturating_sub(1).min(latencies.len() - 1)]
    }
}

impl HttpStressClient {
    pub async fn new(
        base_url: &str,
        token: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::builder()
            .pool_max_idle_per_host(0)
            .tcp_keepalive(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.to_string(),
            token: token.map(String::from),
        })
    }

    pub async fn request(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<&str>,
    ) -> HttpResult {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), endpoint);
        let start = Instant::now();

        let mut req_builder = match method.to_uppercase().as_str() {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => {
                return HttpResult {
                    latency_ms: start.elapsed().as_millis() as u64,
                    status: 0,
                    error: Some(format!("Unknown method: {}", method)),
                }
            }
        };

        if let Some(t) = &self.token {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", t));
        }

        if let Some(b) = body {
            req_builder = req_builder
                .header("Content-Type", "application/json")
                .body(b.to_string());
        }

        match req_builder.send().await {
            Ok(resp) => HttpResult {
                latency_ms: start.elapsed().as_millis() as u64,
                status: resp.status().as_u16(),
                error: None,
            },
            Err(e) => HttpResult {
                latency_ms: start.elapsed().as_millis() as u64,
                status: 0,
                error: Some(e.to_string()),
            },
        }
    }

    pub async fn stress_test(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<String>,
        concurrency: usize,
        duration_secs: u64,
    ) -> Result<StressTestResults, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let counter = Arc::new(AtomicU64::new(0));

        let (tx, rx) = std::sync::mpsc::channel();

        // Spawn workers
        let handles: Vec<_> = (0..concurrency)
            .map(|_| {
                let client = self.clone();
                let tx = tx.clone();
                let counter = Arc::clone(&counter);
                let method = method.to_string();
                let endpoint = endpoint.to_string();
                let body = body.clone();

                tokio::spawn(async move {
                    while start_time.elapsed().as_secs() < duration_secs {
                        let result = client
                            .request(&method, &endpoint, body.as_deref())
                            .await;
                        counter.fetch_add(1, Ordering::Relaxed);
                        let _ = tx.send(result);
                    }
                })
            })
            .collect();

        // Collect results
        let mut all_results = Vec::new();
        while let Ok(result) = rx.recv_timeout(Duration::from_millis(100)) {
            all_results.push(result);
        }

        // Wait for workers
        for h in handles {
            let _ = h.await;
        }

        // Drain remaining results
        while let Ok(result) = rx.try_recv() {
            all_results.push(result);
        }

        Ok(StressTestResults {
            total_requests: counter.load(Ordering::Relaxed),
            total_duration_ms: start_time.elapsed().as_millis() as u64,
            results: all_results,
        })
    }
}

impl Clone for HttpStressClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            token: self.token.clone(),
        }
    }
}
