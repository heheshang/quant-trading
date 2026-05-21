use serde::Serialize;

use super::http_client::StressTestResults;

#[derive(Debug, Serialize)]
pub struct HttpReport {
    pub scenario: String,
    pub qps: f64,
    pub avg_latency_ms: f64,
    pub p50_ms: u64,
    pub p90_ms: u64,
    pub p99_ms: u64,
    pub p999_ms: u64,
    pub min_ms: u64,
    pub max_ms: u64,
    pub total_requests: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub error_rate: f64,
    pub pass: bool,
}

impl HttpReport {
    pub fn from_results(scenario: &str, results: &StressTestResults) -> Self {
        let latencies: Vec<u64> = results.results.iter().map(|r| r.latency_ms).collect();
        let success_count = results.results.iter().filter(|r| r.status >= 200 && r.status < 400).count() as u64;
        let error_count = results.results.len() as u64 - success_count;
        let error_rate = if results.results.is_empty() {
            0.0
        } else {
            error_count as f64 / results.results.len() as f64
        };

        let mut sorted = latencies.clone();
        sorted.sort();

        let p = |pct: f64| -> u64 {
            if sorted.is_empty() {
                return 0;
            }
            let idx = ((sorted.len() as f64) * pct).floor() as usize;
            sorted[idx.min(sorted.len() - 1)]
        };

        let qps = if results.total_duration_ms > 0 {
            results.total_requests as f64 / (results.total_duration_ms as f64 / 1000.0)
        } else {
            0.0
        };

        let avg_ms = if results.results.is_empty() {
            0.0
        } else {
            results.results.iter().map(|r| r.latency_ms as f64).sum::<f64>() / results.results.len() as f64
        };

        let pass = qps >= 500.0 && p(0.99) <= 200 && error_rate <= 0.01;

        Self {
            scenario: scenario.to_string(),
            qps,
            avg_latency_ms: avg_ms,
            p50_ms: p(0.50),
            p90_ms: p(0.90),
            p99_ms: p(0.99),
            p999_ms: p(0.999),
            min_ms: sorted.first().copied().unwrap_or(0),
            max_ms: sorted.last().copied().unwrap_or(0),
            total_requests: results.total_requests,
            success_count,
            error_count,
            error_rate,
            pass,
        }
    }
}

pub struct ReportGenerator;

impl ReportGenerator {
    pub fn generate_http_report(scenario: &str, results: &StressTestResults) -> HttpReport {
        HttpReport::from_results(scenario, results)
    }
}
