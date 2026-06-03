//! mq/ — P3-2 RabbitMQ 客户端封装
//!
//! 中文：lapin 客户端 + deadpool-lapin 连接池。给上层（publisher / consumer / workers）一个
//!   简洁的同步/异步 API；失败兜底回退到原同步路径（MQ_ENABLED=false）。
//!
//! English: thin wrapper over lapin + deadpool-lapin. Exposes async publish + sync
//!   consumer registration. When `MQ_ENABLED=false` the whole module is short-circuited
//!   to keep the prior synchronous behaviour.
//!
//! ## 设计要点（Design notes）
//!
//! - **Connection pool**：`deadpool-lapin::Pool` 持有 N 个 lapin Connection，
//!   publisher 取一个、release 时自动归还。消费者每个 queue 单独走 `Connection::create_channel`。
//! - **Queue 拓扑**：本模块只认 3 个 internal queue（见 [`queues`]），所有 exchange/routing-key
//!   在此 module 内集中维护，避免散落。
//! - **幂等声明**：publisher/consumer 启动时都会重新 `queue_declare`，参数一致 → 重复声明无副作用。
//! - **失败兜底**：publish 失败不会让上游 panic — `MqClient::publish` 返回 `Result<(), String>`，
//!   调用方决定 retry / 同步兜底。**MQ 整体不可用 = 业务降级到同步路径**，不丢数据。
//!
//! ## 用法（Usage）
//!
//! ```no_run
//! use quant_trading_backend::mq::{MqClient, JobKind, queues};
//!
//! # async fn demo() -> Result<(), String> {
//! let client = MqClient::connect("amqp://localhost:5672/%2f").await?;
//! client
//!     .publish(JobKind::AiPrediction, &serde_json::json!({"symbol": "BTCUSDT"}))
//!     .await?;
//! # Ok(()) }
//! ```

pub mod consumer;
pub mod publisher;
pub mod queues;

pub use consumer::{Consumer, ConsumerConfig, spawn_consumer};
pub use publisher::MqClient;
pub use queues::{JobKind, QUEUE_DECLARATIONS, QueueDeclaration};

use std::time::Duration;

/// 默认 AMQP 端口 + 路径前缀。便于测试和本地裸跑。
pub const DEFAULT_AMQP_PORT: u16 = 5672;

/// 连接超时：10s — 与 `reqwest` 默认对齐。重启后第一次连接可能在重连循环中。
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// 默认 channel QoS prefetch — consumer 一次最多预取的消息数。
///   数值偏小（10）保证 worker 间负载相对均匀。
pub const DEFAULT_PREFETCH: u16 = 10;

/// 重连指数退避上限 — 30s。低于 30s 在 RabbitMQ 重启期间会被打爆。
pub const MAX_RECONNECT_BACKOFF: Duration = Duration::from_secs(30);
