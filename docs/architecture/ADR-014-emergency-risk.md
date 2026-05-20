# ADR-014: 系统应急风控 (P0-F3)

## 状态
已实现（2026-05-19）

## 背景
P0-F3 PRD 要求三大应急保护机制，对应上线验收标准：
1. WebSocket 断连超过 N 秒自动禁止开仓（默认 30s）
2. 后端进程异常退出时，系统自动平仓保护（依赖外部看门狗）
3. 紧急一键全平（管理员手动，不受风控规则限制）

## 技术方案

### 架构
- **断线检测**：WsHub 维护 `last_heartbeat: AtomicU64`，`run_connector` 每收到 Binance 消息时更新
- **策略暂停标志**：`strategy_paused_by_disconnect: AtomicBool`，断线超阈值时设为 true
- **心跳检测**：每秒轮询 `last_heartbeat`，超阈值触发 `HubEvent::Disconnected`
- **重连恢复**：Binance WebSocket 重连成功时，`strategy_paused_by_disconnect` 重置为 false，发送 `HubEvent::Connected`
- **连接状态 API**：`GET /api/v1/risk/connection-status` 返回实时状态（exchange_connected, disconnect_elapsed_secs, strategy_paused）

### 核心字段（ws_hub.rs WsHub struct）
```rust
last_heartbeat: Arc<AtomicU64>,              // 上次收到 Binance 消息的时间戳
disconnect_threshold_secs: Arc<AtomicU64>,    // 断线判定阈值（默认 30s）
strategy_paused_by_disconnect: Arc<AtomicBool>, // 是否因断线触发自动暂停
```

### 关键方法
| 方法 | 位置 | 说明 |
|------|------|------|
| `record_heartbeat()` | ws_hub.rs | 每收到消息时调用，更新心跳时间戳 |
| `is_binance_connected()` | ws_hub.rs | 基于心跳判断连接是否活跃 |
| `check_disconnect_and_pause()` | ws_hub.rs | 断线超阈值时设置暂停标志 |
| `on_binance_reconnect()` | ws_hub.rs | 重连成功时清除暂停标志 |
| `get_connection_status()` | ws_hub.rs | 返回 `ConnectionStatus` 供 API 使用 |
| `connection_status()` | handlers/risk.rs | API 端点，调用 WsHub 真实状态 |

### 数据结构
```rust
pub struct ConnectionStatus {
    pub exchange_connected: bool,        // Binance 连接是否活跃
    pub disconnect_elapsed_secs: u64,    // 距离上次消息的秒数
    pub disconnect_threshold_secs: u64,  // 断线判定阈值
    pub strategy_paused: bool,           // 策略是否因断线被暂停
}
```

## API 端点

### GET /api/v1/risk/connection-status
- 认证：需要 AuthenticatedUser
- 依赖：WsHub Extension（由 main.rs 注入）
- 返回：`ConnectionStatus`（真实心跳状态）

### 已有端点（复用）
- `POST /api/v1/risk/pause` — 手动暂停交易（P0-F2）
- `POST /api/v1/risk/resume` — 手动恢复交易（P0-F2）
- `POST /api/v1/risk/emergency-close` — 紧急全平（P0-F2）

## 已知限制
1. **宕机平仓**：依赖外部看门狗（supervisord/systemd）检测进程退出并触发平仓脚本。本模块仅提供 `emergency-close` API，不负责进程监控。
2. **策略联动**：当前仅设置 `strategy_paused_by_disconnect` 标志，实际策略执行引擎尚未接入此标志（待 P1 阶段完成策略状态机）。
3. **重连后自动恢复**：在 Binance WebSocket 重连成功后，策略自动恢复。若需要"重连稳定 N 秒后才恢复"，需在 `on_binance_reconnect` 后增加额外定时器。

## 文件变更
| 文件 | 变更 |
|------|------|
| `src/services/exchange/ws_hub.rs` | 新增断线检测字段、ConnectionStatus、心跳方法、断线检测逻辑 |
| `src/handlers/risk.rs` | connection_status 改用真实 WsHub 状态，移除本地 ConnectionStatus 定义 |
| `src/main.rs` | risk::router() 添加 Extension(ws_hub) layer |
