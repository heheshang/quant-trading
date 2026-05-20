//! Order Rate Limiter — 多维度滑动窗口频率限制
//!
//! PRD: P1-F4
//! - 单用户频率限制：每分钟最大下单数
//! - 单交易对频率限制：每个交易对每分钟最大下单数
//! - 全局频率限制：整个系统每分钟最大下单总数
//!
//! 限制策略：超限时返回 429 Too Many Requests，错误码 "RL-001"

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::warn;
use uuid::Uuid;

/// 默认限制值
const DEFAULT_USER_LIMIT: u64 = 10; // 每用户每分钟10单
const DEFAULT_SYMBOL_LIMIT: u64 = 20; // 每交易对每分钟20单
const DEFAULT_GLOBAL_LIMIT: u64 = 100; // 全局每分钟100单
const WINDOW_SECS: u64 = 60;

/// 单维度滑动窗口计数器
#[derive(Clone)]
struct SlidingWindow {
    requests: VecDeque<i64>, // 毫秒时间戳队列
}

impl SlidingWindow {
    fn new() -> Self {
        Self {
            requests: VecDeque::new(),
        }
    }

    /// 检查是否超限，超限返回 false
    fn check(&mut self, now_ms: i64, max_req: u64) -> bool {
        let window_start = now_ms - (WINDOW_SECS * 1000) as i64;

        // 移除窗口外的老请求
        while self.requests.front().is_some_and(|&ts| ts <= window_start) {
            self.requests.pop_front();
        }

        if self.requests.len() >= max_req as usize {
            return false; // 超限
        }
        self.requests.push_back(now_ms);
        true
    }

    /// 获取当前窗口内的请求数
    fn count(&self, now_ms: i64) -> usize {
        let window_start = now_ms - (WINDOW_SECS * 1000) as i64;
        self.requests
            .iter()
            .filter(|&&ts| ts > window_start)
            .count()
    }

    /// 获取剩余额度
    fn remaining(&self, now_ms: i64, max_req: u64) -> u64 {
        max_req.saturating_sub(self.count(now_ms) as u64)
    }
}

impl Default for SlidingWindow {
    fn default() -> Self {
        Self::new()
    }
}

/// 频率限制检查结果
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u64,
    pub remaining: u64,
    pub reset_ms: i64,
}

/// 订单频率限制器 — 支持多维度计数
#[derive(Clone)]
pub struct OrderRateLimiter {
    /// 用户维度限制器: user_id -> SlidingWindow
    user_limits: Arc<RwLock<HashMap<Uuid, SlidingWindow>>>,
    /// 交易对维度限制器: symbol -> SlidingWindow
    symbol_limits: Arc<RwLock<HashMap<String, SlidingWindow>>>,
    /// 全局维度限制器
    global_limit: Arc<RwLock<SlidingWindow>>,
    /// 用户限制数
    user_limit: u64,
    /// 交易对限制数
    symbol_limit: u64,
    /// 全局限制数
    global_limit_count: u64,
}

impl OrderRateLimiter {
    /// 创建新的 OrderRateLimiter，使用默认限制值
    pub fn new() -> Self {
        Self::with_limits(
            DEFAULT_USER_LIMIT,
            DEFAULT_SYMBOL_LIMIT,
            DEFAULT_GLOBAL_LIMIT,
        )
    }

    /// 创建 OrderRateLimiter，自定义限制值
    pub fn with_limits(user_limit: u64, symbol_limit: u64, global_limit: u64) -> Self {
        Self {
            user_limits: Arc::new(RwLock::new(HashMap::new())),
            symbol_limits: Arc::new(RwLock::new(HashMap::new())),
            global_limit: Arc::new(RwLock::new(SlidingWindow::new())),
            user_limit,
            symbol_limit,
            global_limit_count: global_limit,
        }
    }

    /// 检查并记录订单请求
    ///
    /// 返回 RateLimitInfo 或超限时返回错误
    pub async fn check(
        &self,
        user_id: Uuid,
        symbol: &str,
    ) -> Result<RateLimitInfo, crate::utils::error::AppError> {
        use crate::utils::error::AppError;

        let now_ms = chrono::Utc::now().timestamp_millis();

        // 1. 检查用户维度
        {
            let mut users = self.user_limits.write().await;
            let limiter = users.entry(user_id).or_insert_with(SlidingWindow::new);
            if !limiter.check(now_ms, self.user_limit) {
                warn!(
                    user_id = %user_id,
                    "User order rate limit exceeded: {}/{}",
                    limiter.count(now_ms),
                    self.user_limit
                );
                return Err(AppError::TooManyRequests(
                    "RL-001: Order rate limit exceeded".into(),
                ));
            }
        }

        // 2. 检查交易对维度
        {
            let mut symbols = self.symbol_limits.write().await;
            let limiter = symbols
                .entry(symbol.to_string())
                .or_insert_with(SlidingWindow::new);
            if !limiter.check(now_ms, self.symbol_limit) {
                warn!(
                    symbol = %symbol,
                    "Symbol order rate limit exceeded: {}/{}",
                    limiter.count(now_ms),
                    self.symbol_limit
                );
                return Err(AppError::TooManyRequests(
                    "RL-001: Order rate limit exceeded".into(),
                ));
            }
        }

        // 3. 检查全局维度
        {
            let mut global = self.global_limit.write().await;
            if !global.check(now_ms, self.global_limit_count) {
                warn!(
                    "Global order rate limit exceeded: {}/{}",
                    global.count(now_ms),
                    self.global_limit_count
                );
                return Err(AppError::TooManyRequests(
                    "RL-001: Order rate limit exceeded".into(),
                ));
            }
        }

        // 4. 计算剩余额度（取三个维度的最小剩余）
        let user_remaining = {
            let users = self.user_limits.read().await;
            users
                .get(&user_id)
                .map(|l| l.remaining(now_ms, self.user_limit))
                .unwrap_or(self.user_limit)
        };

        let symbol_remaining = {
            let symbols = self.symbol_limits.read().await;
            symbols
                .get(symbol)
                .map(|l| l.remaining(now_ms, self.symbol_limit))
                .unwrap_or(self.symbol_limit)
        };

        let global_remaining = {
            let global = self.global_limit.read().await;
            global.remaining(now_ms, self.global_limit_count)
        };

        let remaining = user_remaining.min(symbol_remaining).min(global_remaining);
        let reset_ms = now_ms + (WINDOW_SECS * 1000) as i64;

        Ok(RateLimitInfo {
            limit: remaining,
            remaining,
            reset_ms,
        })
    }

    /// 获取当前用户的频率限制信息（不记录请求）
    pub async fn get_info(&self, user_id: Uuid, symbol: &str) -> RateLimitInfo {
        let now_ms = chrono::Utc::now().timestamp_millis();

        let user_remaining = {
            let users = self.user_limits.read().await;
            users
                .get(&user_id)
                .map(|l| l.remaining(now_ms, self.user_limit))
                .unwrap_or(self.user_limit)
        };

        let symbol_remaining = {
            let symbols = self.symbol_limits.read().await;
            symbols
                .get(symbol)
                .map(|l| l.remaining(now_ms, self.symbol_limit))
                .unwrap_or(self.symbol_limit)
        };

        let global_remaining = {
            let global = self.global_limit.read().await;
            global.remaining(now_ms, self.global_limit_count)
        };

        let remaining = user_remaining.min(symbol_remaining).min(global_remaining);

        RateLimitInfo {
            limit: remaining,
            remaining,
            reset_ms: now_ms + (WINDOW_SECS * 1000) as i64,
        }
    }

    /// 获取用户限制数
    pub fn user_limit(&self) -> u64 {
        self.user_limit
    }

    /// 获取交易对限制数
    pub fn symbol_limit(&self) -> u64 {
        self.symbol_limit
    }

    /// 获取全局限制数
    pub fn global_limit(&self) -> u64 {
        self.global_limit_count
    }
}

impl Default for OrderRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// ─── 单元测试 ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_limiter() -> OrderRateLimiter {
        OrderRateLimiter::with_limits(10, 20, 100)
    }

    #[tokio::test]
    async fn test_normal_order_success() {
        let limiter = create_test_limiter();
        let user_id = Uuid::new_v4();
        let symbol = "BTC/USDT";

        // 前10单应该成功
        for i in 0..10 {
            let result = limiter.check(user_id, symbol).await;
            assert!(result.is_ok(), "Order {} should succeed", i + 1);
        }
    }

    #[tokio::test]
    async fn test_user_limit_exceeded() {
        let limiter = create_test_limiter();
        let user_id = Uuid::new_v4();
        let symbol = "BTC/USDT";

        // 下10单成功
        for _ in 0..10 {
            limiter.check(user_id, symbol).await.unwrap();
        }

        // 第11单应该被拒绝
        let result = limiter.check(user_id, symbol).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::utils::error::AppError::TooManyRequests(_)
        ));
    }

    #[tokio::test]
    async fn test_symbol_limit_independent() {
        // Use separate users to avoid user limit interfering with symbol limit test
        let limiter = OrderRateLimiter::with_limits(100, 20, 1000);
        let user_id = Uuid::new_v4();
        let btc_symbol = "BTC/USDT";
        let eth_symbol = "ETH/USDT";

        // BTC user1 下满20单
        for _ in 0..20 {
            limiter.check(user_id, btc_symbol).await.unwrap();
        }

        // BTC 第21单应该被拒绝（symbol limit）
        let result = limiter.check(user_id, btc_symbol).await;
        assert!(result.is_err());

        // ETH 下单应该成功（独立计算）
        let result = limiter.check(user_id, eth_symbol).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_global_limit() {
        // 使用更小的全局限制测试
        let limiter = OrderRateLimiter::with_limits(100, 100, 5);
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();
        let symbol = "BTC/USDT";

        // user1 下5单
        for _ in 0..5 {
            limiter.check(user1, symbol).await.unwrap();
        }

        // user1 第6单应该被全局限制拒绝
        let result = limiter.check(user1, symbol).await;
        assert!(result.is_err());

        // user2 的下单也应该被全局限制拒绝
        let result = limiter.check(user2, symbol).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_different_users_independent() {
        let limiter = create_test_limiter();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();
        let symbol = "BTC/USDT";

        // user1 下满10单
        for _ in 0..10 {
            limiter.check(user1, symbol).await.unwrap();
        }

        // user1 第11单应该被拒绝
        let result = limiter.check(user1, symbol).await;
        assert!(result.is_err());

        // user2 下单应该成功（独立计算）
        let result = limiter.check(user2, symbol).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_sliding_window_basic() {
        let mut window = SlidingWindow::new();
        let now_ms = 1000000i64;

        // 前5次应该成功
        for i in 0..5 {
            assert!(window.check(now_ms + i as i64 * 100, 10));
        }

        // 第6次应该失败（超过5个）
        assert!(!window.check(now_ms + 600, 5));
    }

    #[test]
    fn test_sliding_window_old_entries_removed() {
        let mut window = SlidingWindow::new();
        let now_ms = 1000000i64;

        // 下5单
        for i in 0..5 {
            window.check(now_ms + i as i64 * 100, 10);
        }

        // 60秒后（窗口外）
        let later = now_ms + 60000 + 500;
        assert!(window.check(later, 5)); // 旧记录应该被清除，可以再下5单
    }

    #[test]
    fn test_sliding_window_remaining() {
        let mut window = SlidingWindow::new();
        let now_ms = 1000000i64;

        // 下3单
        for i in 0..3 {
            window.check(now_ms + i as i64 * 100, 10);
        }

        assert_eq!(window.remaining(now_ms + 300, 10), 7);
    }
}
