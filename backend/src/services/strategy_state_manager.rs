//! StrategyStateManager — 断线暂停后台任务 (F6)
//!
//! ADR-016: DisconnectionMonitor 后台任务，每 5s 检查一次 WsHub 连接状态
//! - 断线 > 30s → RiskManager.pause()
//! - 重连稳定 10s → RiskManager.resume()

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::services::exchange::ws_hub::WsHub;
use crate::services::risk_manager::RiskManager;

/// 全局唯一实例（通过 Arc 共享）
pub struct StrategyStateManager {
    /// WsHub 引用
    ws_hub: Arc<WsHub>,
    /// RiskManager 引用
    risk_manager: Arc<RiskManager>,
    /// 上次触发暂停的断线时长（用于避免重复触发）
    last_pause_disconnect_secs: Mutex<u64>,
    /// 稳定重连计数器（连续重连秒数，达到 10s 才恢复）
    reconnect_stable_secs: Mutex<u64>,
}

impl StrategyStateManager {
    /// 创建新的 StrategyStateManager
    pub fn new(ws_hub: Arc<WsHub>, risk_manager: Arc<RiskManager>) -> Self {
        Self {
            ws_hub,
            risk_manager,
            last_pause_disconnect_secs: Mutex::new(0),
            reconnect_stable_secs: Mutex::new(0),
        }
    }

    /// 启动后台监控任务（每 5s 检查一次）
    pub fn start(self: Arc<Self>) {
        let manager = self.clone();
        tokio::spawn(async move {
            manager.run_monitor().await;
        });
        info!("StrategyStateManager started (F6 disconnection monitor)");
    }

    /// 后台任务主循环：每 5s 检查一次连接状态
    async fn run_monitor(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(5));
        // 系统默认 UUID（断线暂停使用系统级标识）
        let system_user_id = uuid::Uuid::nil();

        loop {
            ticker.tick().await;
            self.check_and_update_state(system_user_id).await;
        }
    }

    /// 检查连接状态并更新策略状态
    async fn check_and_update_state(self: &Arc<Self>, system_user_id: uuid::Uuid) {
        let status = self.ws_hub.get_connection_status();
        let is_connected = status.exchange_connected;
        let disconnect_elapsed = status.disconnect_elapsed_secs;
        let threshold = status.disconnect_threshold_secs;

        if !is_connected && disconnect_elapsed > threshold {
            // ── 断线超过 30s：触发暂停 ──
            let mut last_pause = self.last_pause_disconnect_secs.lock().await;
            let disconnect_gap = disconnect_elapsed;

            if *last_pause == 0 || *last_pause < disconnect_gap {
                warn!(
                    "Binance disconnected for {}s (threshold: {}s), pausing strategies",
                    disconnect_gap, threshold
                );
                match self
                    .risk_manager
                    .pause(
                        system_user_id,
                        &format!("断线 {}s 超过阈值 {}s 自动暂停", disconnect_gap, threshold),
                    )
                    .await
                {
                    Ok(_) => {
                        info!("RiskManager.pause() called successfully");
                        *last_pause = disconnect_gap;
                        // 重置稳定重连计数器
                        let mut stable = self.reconnect_stable_secs.lock().await;
                        *stable = 0;
                    }
                    Err(e) => {
                        error!("Failed to call RiskManager.pause(): {:?}", e);
                    }
                }
            }
        } else if is_connected {
            // ── 重连稳定：连续 10s 保持连接则恢复 ──
            let mut stable = self.reconnect_stable_secs.lock().await;
            *stable += 5; // 每次 tick 增加 5s

            if *stable >= 10 {
                let mut last_pause = self.last_pause_disconnect_secs.lock().await;
                if *last_pause > 0 {
                    info!("Binance reconnected stably for 10s, resuming strategies");
                    match self.risk_manager.resume(system_user_id).await {
                        Ok(_) => {
                            info!("RiskManager.resume() called successfully");
                            *last_pause = 0;
                            *stable = 0;
                        }
                        Err(e) => {
                            error!("Failed to call RiskManager.resume(): {:?}", e);
                        }
                    }
                } else {
                    // 已恢复，重置计数器
                    *stable = 0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_manager_creation() {
        // Basic test to ensure the module compiles
        // Integration tests would require mocked WsHub and RiskManager
        assert!(true);
    }
}
