//! mq/publisher.rs — `MqClient` 异步 publisher
//!
//! 设计要点：
//!   - **Pool-based**：内部持 `deadpool_lapin::Pool`，每次 publish 借用一个 connection，
//!     完成后归还。N=4 在 200 RPS publish 量级下基本不会争抢。
//!   - **Channel per publish**：每次取 connection 后 `create_channel()`；lapin 的 channel
//!     是「廉价对象」，复用会污染 ack 状态。
//!   - **JSON envelope**：`publish` 接受任意 `Serialize`，内部裹成 `{kind, payload, ts}`，
//!     consumer 端从 `data.kind` 拿到 JobKind 反查。
//!   - **Priority**：`JobKind::AiPrediction` 自动打最高 priority (10)，其余走 default 0。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use deadpool_lapin::{Manager, Pool, PoolConfig, Status};
use lapin::options::{BasicPublishOptions, ConfirmSelectOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties};
use serde::{Deserialize, Serialize};

use crate::metrics;
use crate::mq::queues::{JobKind, declare_all_queues};

/// Publisher 错误类型。**不** 透传 lapin 内部错误 — 调用方只关心能否 publish 成功，
///   细节 log 里有。
#[derive(Debug, thiserror::Error)]
pub enum MqError {
    /// 连接池耗尽（deadpool 会自动回收，重试一次通常就够）
    #[error("connection pool exhausted: {0}")]
    PoolExhausted(String),
    /// 拉取/创建 channel 失败
    #[error("channel error: {0}")]
    Channel(String),
    /// broker 返回 nack / 通道关闭
    #[error("broker rejected publish: {0}")]
    Broker(String),
    /// JSON 序列化失败（调用方传了不可序列化的 payload）
    #[error("serialize envelope: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, MqError>;

/// 队列中传输的 JSON 信封。
///
/// 字段命名用 snake_case 跟全系统一致（CLAUDE.md 约定）。
/// `payload` 是真正的业务数据，结构由上游业务方决定（worker 反序列化时再校验）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<P> {
    /// job 类别 — 跟 JobKind 对应，consumer 用它做 first-pass 路由
    pub kind: &'static str,
    /// 业务 payload。type-erased 的话用 `serde_json::Value`。
    pub payload: P,
    /// unix timestamp (ms) — broker 不会改这个时间戳，便于 replay/审计
    pub enqueued_at_ms: u128,
    /// 单调递增 id — 由 publisher 进程内 atomic 生成，跨重启可能重复
    ///   （worker 端应该幂等处理，详见各 worker 的注释）
    pub job_id: u64,
}

impl<P: Serialize> Envelope<P> {
    pub fn new(kind: JobKind, payload: P, job_id: u64) -> Self {
        Self {
            kind: kind.as_str(),
            payload,
            enqueued_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            job_id,
        }
    }
}

/// 全异步 client — clone 后共享同一个 pool。
#[derive(Clone)]
pub struct MqClient {
    pool: Arc<Pool>,
    /// 进程内单调 job id 源
    next_job_id: Arc<std::sync::atomic::AtomicU64>,
}

impl std::fmt::Debug for MqClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqClient")
            .field("pool_status", &self.pool.status())
            .finish()
    }
}

impl MqClient {
    /// 用 AMQP URL 建一个 publisher pool。
    ///
    /// 注意：**失败不会 panic** — 在 broker 启动顺序乱的情况下希望进程先起，
    ///   publish 时再按需报错。
    pub async fn connect(amqp_url: &str) -> Result<Self> {
        let manager = Manager::new(amqp_url.to_string(), ConnectionProperties::default());
        let pool = Pool::builder(manager)
            .max_size(8)
            .config(PoolConfig::default())
            .build()
            .map_err(|e| MqError::PoolExhausted(e.to_string()))?;

        // 立即 try 一个 connection（不会真起 channel）。失败 → 仍然返回 client，
        //   业务 publish 第一次才会真正感知。
        if let Err(e) = pool.get().await {
            tracing::warn!("MqClient: initial pool warmup failed: {e}");
        }

        Ok(Self {
            pool: Arc::new(pool),
            next_job_id: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    /// 测试/兜底用：构造一个永远失败的 client（不连 broker）。
    /// 典型用法：`MQ_ENABLED=false` 时注入，调用方 publish 立刻报错 → 走同步兜底。
    pub fn disconnected() -> Self {
        let manager = Manager::new(
            // deadpool 不会立即连，等到 get().await 才连。所以这个 URL 永远连不上即可。
            "amqp://disabled:disabled@127.0.0.1:1/%2f".to_string(),
            ConnectionProperties::default(),
        );
        let pool = Pool::builder(manager)
            .max_size(1)
            .config(PoolConfig::default())
            .build()
            .expect("pool build with disabled URL is infallible");
        Self {
            pool: Arc::new(pool),
            next_job_id: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Publisher confirms（`confirm.select`）— 拿到 `Confirmation` 才算成功。
    /// 业务侧不需要区分 ack/nack 细节：nack 走 `MqError::Broker` 抛出。
    async fn publish_with_confirm(
        &self,
        routing_key: &str,
        body: Vec<u8>,
        priority: u8,
    ) -> Result<()> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| MqError::PoolExhausted(e.to_string()))?;
        let channel: Channel = conn
            .create_channel()
            .await
            .map_err(|e| MqError::Channel(e.to_string()))?;
        // enable confirm mode（防止 fire-and-forget 丢消息）
        channel
            .confirm_select(ConfirmSelectOptions::default())
            .await
            .map_err(|e| MqError::Channel(format!("confirm_select: {e}")))?;

        let props = BasicProperties::default()
            .with_delivery_mode(2) // persistent
            .with_content_type("application/json".into())
            .with_priority(priority);

        // default exchange + routing_key = queue name
        let confirm = channel
            .basic_publish(
                "",
                routing_key,
                BasicPublishOptions::default(),
                &body,
                props,
            )
            .await
            .map_err(|e| MqError::Broker(e.to_string()))?
            .await
            .map_err(|e| MqError::Broker(format!("await confirmation: {e}")))?;

        if confirm.is_nack() {
            return Err(MqError::Broker("broker returned nack".into()));
        }
        Ok(())
    }

    /// 高层 API：把任意 `Serialize` payload 投到指定 queue。
    ///
    /// 失败语义：**调用方需要决定 retry / 同步兜底**。本方法不会自动重试 —
    ///   重试可能导致上游 4xx 路径上重复发送（应交给 worker 端 idempotency 兜底）。
    pub async fn publish<P: Serialize>(
        &self,
        kind: JobKind,
        payload: &P,
    ) -> Result<u64> {
        let job_id = self
            .next_job_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let envelope = Envelope::new(kind, payload, job_id);
        let body = serde_json::to_vec(&envelope)?;

        let priority = match kind {
            JobKind::AiPrediction => 10,
            _ => 0,
        };

        match self
            .publish_with_confirm(kind.routing_key(), body, priority)
            .await
        {
            Ok(()) => {
                metrics::MQ_PUBLISH_TOTAL
                    .with_label_values(&[kind.as_str()])
                    .inc();
                Ok(job_id)
            }
            Err(e) => {
                metrics::MQ_PUBLISH_FAILED_TOTAL
                    .with_label_values(&[kind.as_str()])
                    .inc();
                Err(e)
            }
        }
    }

    /// 给 startup 时一次性 declare 所有 queue 用 — 共享 helper。
    pub async fn declare_queues(&self) -> Result<()> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| MqError::PoolExhausted(e.to_string()))?;
        let channel = conn
            .create_channel()
            .await
            .map_err(|e| MqError::Channel(e.to_string()))?;
        declare_all_queues(&channel)
            .await
            .map_err(|e| MqError::Channel(format!("declare_all_queues: {e}")))
    }

    /// 报告连接池状态（用于 health 端点）。
    pub fn pool_status(&self) -> Status {
        self.pool.status()
    }

    /// 轻量 ping — `pool.get()` 一次。不 publish 任何消息，不影响业务。
    pub async fn ping(&self) -> Result<()> {
        let _conn: deadpool_lapin::Object = self
            .pool
            .get()
            .await
            .map_err(|e| MqError::PoolExhausted(e.to_string()))?;
        Ok(())
    }
}

/// 启动时的一次性 helper：建连接 + declare queues + 留个 client 在 Arc 里。
///
/// 返回 None 的场景：
///   1. `MQ_ENABLED=false`
///   2. 连接 broker 失败（启动期允许，运行时 publish 会按需报）
pub async fn init_mq_client(amqp_url: &str, mq_enabled: bool) -> Option<MqClient> {
    if !mq_enabled {
        tracing::info!("MQ disabled via MQ_ENABLED=false; using disconnected stub");
        return None;
    }
    match MqClient::connect(amqp_url).await {
        Ok(c) => {
            if let Err(e) = c.declare_queues().await {
                tracing::warn!("MQ declare_queues failed at startup: {e}");
            } else {
                tracing::info!("MQ ready: {amqp_url}");
            }
            Some(c)
        }
        Err(e) => {
            tracing::error!("MQ connect failed: {e}");
            None
        }
    }
}

// Re-export `Connection` for tests / type hints even though most callers use the pool.
pub type LapinConnection = Connection;
#[allow(dead_code)]
pub fn empty_field_table() -> FieldTable {
    FieldTable::default()
}
