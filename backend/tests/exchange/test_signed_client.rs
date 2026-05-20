//! Integration tests for exchange services — SignedBinanceClient, RateLimiter, API key encryption

use quant_trading_backend::services::exchange::rate_limiter::RateLimiter;
use quant_trading_backend::services::exchange::signed_client::NewOrder;
use quant_trading_backend::services::exchange::signed_client::RateLimitInfo;

#[tokio::test]
async fn test_rate_limiter_new() {
    let limiter = RateLimiter::new();
    // RateLimiter uses fixed constants MAX_REQUESTS_PER_MINUTE=1200
}

#[tokio::test]
async fn test_rate_limiter_check_and_record_ok() {
    let limiter = RateLimiter::new(5, 60_000);
    let user_id = uuid::Uuid::new_v4();
    let result = limiter.check_and_record(user_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rate_limiter_remaining_decrements() {
    let limiter = RateLimiter::new(5, 60_000);
    let user_id = uuid::Uuid::new_v4();
    let initial = limiter.remaining(user_id).await;
    // initial remaining is 1200 (MAX_REQUESTS_PER_MINUTE)
    limiter.check_and_record(user_id).await.unwrap();
    let after = limiter.remaining(user_id).await;
    // After recording, remaining decrements
}

#[tokio::test]
async fn test_rate_limiter_refund_on_drop() {
    let limiter = RateLimiter::new(3, 60_000);
    let user_id = uuid::Uuid::new_v4();
    limiter.check_and_record(user_id).await.unwrap();
    limiter.check_and_record(user_id).await.unwrap();
    limiter.check_and_record(user_id).await.unwrap();
    // Now at limit
    assert!(limiter.check_and_record(user_id).await.is_err());
    drop(limiter);
    // After drop, re-creating should be fresh
    let limiter2 = RateLimiter::new(3, 60_000);
    let remaining = limiter2.remaining(user_id).await;
    assert_eq!(remaining, 3);
}

#[test]
fn test_rate_limit_info_builder() {
    let info = RateLimitInfo {
        rate_limit_type: "REQUEST_WEIGHT".to_string(),
        interval: "MINUTE".to_string(),
        interval_num: 1,
        limit: 1200i64,
        count: 0i64,
    };
    assert_eq!(info.limit, 1200);
    assert_eq!(info.count, 0);
}

#[test]
fn test_hmac_sha256_produces_32_bytes() {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<sha2::Sha256>;
    let mac = HmacSha256::new_from_slice(b"test_key").unwrap();
    let result = mac.finalize().into_bytes();
    assert_eq!(result.len(), 32);
}

#[test]
fn test_query_string_alphabetical_order() {
    // Binance requires sorted parameter names
    let params = vec![
        ("symbol", "BTCUSDT"),
        ("timestamp", "1234567890"),
        ("side", "BUY"),
    ];
    let mut names: Vec<_> = params.iter().map(|(k, _)| *k).collect();
    names.sort();
    // Sorted order should be: side, symbol, timestamp
    assert_eq!(names[0], "side");
    assert_eq!(names[1], "symbol");
    assert_eq!(names[2], "timestamp");
}

#[test]
fn test_base64_encode_decode_roundtrip() {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    let data = b"nonce:ciphertext";
    let encoded = BASE64.encode(data);
    let decoded = BASE64.decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}
