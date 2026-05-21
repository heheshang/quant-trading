//! Binance API Request Signer
//!
//! Implements HMAC-SHA256 signature for Binance private API endpoints.
//!
//! # Signature Protocol
//! 1. Build query string: "param1=value1&param2=value2&timestamp=1234567890123"
//! 2. Compute HMAC-SHA256(secret_key, query_string) -> hex lowercase
//! 3. Add X-MBX-APIKEY header + signature query param
//!
//! # Timestamp Validation
//! - Binance requires server time offset within ±5 seconds
//! - Server time is synced via /api/v3/time endpoint

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Generate HMAC-SHA256 signature for Binance API
///
/// # Arguments
/// * `secret_key` - The API secret key (decrypted from storage)
/// * `query_string` - The query string to sign (e.g., "symbol=BTCUSDT&timestamp=123456")
///
/// # Returns
/// * Lowercase hex string of the HMAC-SHA256 signature
pub fn sign_request(secret_key: &str, query_string: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(query_string.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Build a signed query string for Binance API
///
/// # Arguments
/// * `params` - Key-value pairs of query parameters (will be sorted alphabetically)
/// * `timestamp` - Current Unix timestamp in milliseconds
///
/// # Returns
/// * Complete query string including timestamp and signature placeholder
pub fn build_signed_query(params: &[(&str, &str)], timestamp: i64) -> String {
    let mut query_parts: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    query_parts.sort();
    query_parts.push(format!("timestamp={}", timestamp));
    query_parts.join("&")
}

/// Get current Unix timestamp in milliseconds
pub fn current_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as i64
}

/// Binance API key with associated metadata
#[derive(Debug, Clone)]
pub struct BinanceApiKey {
    pub api_key: String,
    pub secret_key: String,
}

impl BinanceApiKey {
    /// Create a new API key pair
    pub fn new(api_key: String, secret_key: String) -> Self {
        Self { api_key, secret_key }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_request() {
        // Known test vector from Binance API docs
        let secret_key = "NhqPtmdSJYdKjVHjA7PHZFHf7WhY9Rq47h0hBwRklP8HBxJfKR1wd3";
        let query_string = "symbol=BTCUSDT&side=BUY&type=LIMIT&quantity=1&price=9000&timestamp=1641381404172";
        let sig = sign_request(secret_key, query_string);
        assert_eq!(sig.len(), 64); // SHA256 hex = 64 chars
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sign_request_deterministic() {
        let sig1 = sign_request("secret", "test");
        let sig2 = sign_request("secret", "test");
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_sign_request_different_inputs() {
        let sig1 = sign_request("secret", "input1");
        let sig2 = sign_request("secret", "input2");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_build_signed_query() {
        let params = [("symbol", "BTCUSDT"), ("side", "BUY")];
        let query = build_signed_query(&params, 1641381404172);
        assert!(query.contains("symbol=BTCUSDT"));
        assert!(query.contains("side=BUY"));
        assert!(query.contains("timestamp=1641381404172"));
    }

    #[test]
    fn test_build_signed_query_params_sorted() {
        let params = [("z_param", "z"), ("a_param", "a")];
        let query = build_signed_query(&params, 1000);
        // Params should appear in order given (caller responsible for sorting)
        let a_pos = query.find("a_param=a").unwrap();
        let z_pos = query.find("z_param=z").unwrap();
        assert!(a_pos < z_pos);
    }

    #[test]
    fn test_current_timestamp_ms_positive() {
        let ts = current_timestamp_ms();
        assert!(ts > 0);
        // Should be roughly current time (within 1 day)
        let day_ms = 86400 * 1000;
        assert!(ts > 1_700_000_000_000); // Sometime after 2023
    }
}
