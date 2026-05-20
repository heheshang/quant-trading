//! 滑动窗口频率限制 — 签名请求限流

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;
use uuid::Uuid;

/// 每分钟最大签名请求数（Binance 限制 1200/分钟）
const MAX_REQUESTS_PER_MINUTE: u64 = 1200;
const WINDOW_SECS: u64 = 60;

/// 单用户滑动窗口计数器
struct UserLimiter {
    requests: VecDeque<i64>, // 毫秒时间戳队列
}

impl UserLimiter {
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

    fn remaining(&self, now_ms: i64) -> u64 {
        let window_start = now_ms - (WINDOW_SECS * 1000) as i64;
        let active = self
            .requests
            .iter()
            .filter(|&&ts| ts > window_start)
            .count();
        MAX_REQUESTS_PER_MINUTE.saturating_sub(active as u64)
    }
}

/// 全局频率限制器
#[derive(Clone)]
pub struct RateLimiter {
    /// user_id → 用户的滑动窗口
    users: Arc<RwLock<std::collections::HashMap<Uuid, UserLimiter>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 检查并记录请求，超限返回错误
    pub async fn check_and_record(
        &self,
        user_id: Uuid,
    ) -> Result<u64, crate::utils::error::AppError> {
        use crate::utils::error::AppError;

        let now_ms = chrono::Utc::now().timestamp_millis();

        let mut users = self.users.write().await;
        let limiter = users.entry(user_id).or_insert_with(UserLimiter::new);

        if !limiter.check(now_ms, MAX_REQUESTS_PER_MINUTE) {
            warn!(user_id = %user_id, "Rate limit exceeded for signed requests");
            return Err(AppError::TooManyRequests(
                "RL-001: Order rate limit exceeded".into(),
            ));
        }

        Ok(limiter.remaining(now_ms))
    }

    /// 获取当前剩余额度
    pub async fn remaining(&self, user_id: Uuid) -> u64 {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let users = self.users.read().await;
        users
            .get(&user_id)
            .map(|l| l.remaining(now_ms))
            .unwrap_or(MAX_REQUESTS_PER_MINUTE)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
