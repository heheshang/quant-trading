//! mq/consumer.rs — 通用 consumer 注册器
//!
//! 设计：
//!   - 每个 worker 拿自己的 `Connection`（不是 pool — 消费者一个 queue 一个长连接，
//!     复用 pool 会和 publisher 抢资源且 ack 状态污染）。
//!   - `basic_consume` 拉到一个 `Delivery` stream，`spawn_consumer` 把
//!     stream 拆成 `(delivery, consumer_tag)` 喂给 `Handler` 闭包。
//!   - 闭包返回 `HandlerOutcome::Ack | Retry | Drop`：
//!     - **Ack**：`basic_ack` 提交，job 不再投递
//!     - **Retry**：发回原 queue（用 `basic_nack(requeue=true)`），business 端做退避
//!     - **Drop**：`basic_nack(requeue=false)`，job 丢弃（用于无意义的 poison message）
//!   - Handler **不允许** panic — 一旦 panic 整个 consumer 任务挂掉，需要进程外监控重启。
//!
//! 重连策略：
//!   - 在 `spawn_consumer` 外层包一层 `loop`，channel 断开后 backoff 重连，
//!     上限 30s（见 `MAX_RECONNECT_BACKOFF`）。

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use futures::StreamExt;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicQosOptions};
use lapin::types::FieldTable;
#[allow(unused_imports)]
use lapin::{Channel, Connection, ConnectionProperties};

use crate::metrics;
use crate::mq::MAX_RECONNECT_BACKOFF;

/// Consumer 行为判定 — 由 handler 返回。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerOutcome {
    /// 处理成功，从 queue 移除
    Ack,
    /// 处理失败，重新入队。**注意**：会在队列头堆积（无退避），本系统
    ///   改用 sleep + nack 模拟简单 backoff（见 `spawn_consumer` 内部）。
    ///   真正的指数退避留给业务 worker（通过 job payload 内的 retry 计数实现）。
    Retry,
    /// 永久失败，丢弃（如 deserialize 失败 / 字段缺失）
    Drop,
}

/// Handler 闭包类型 — 接收 (delivery, payload_str) 序列化为 JSON 的字符串，
///   失败时返回 `HandlerOutcome::Drop`，成功时由 worker 自己 `serde_json::from_str`
///   解出业务结构。
///
/// 为什么不直接传 `serde_json::Value`：
///   1. Worker 端结构体往往带 `chrono::DateTime`，多一层结构是绕路。
///   2. 把"原始 body"和"业务结构"解耦 — 测试时可以传 malformed body 验证 Drop 路径。
pub type Handler =
    Box<dyn Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = HandlerOutcome> + Send>> + Send + Sync>;

/// Consumer 配置 — 简化为「queue 名 + handler 闭包 + prefetch」。
#[derive(Clone)]
pub struct ConsumerConfig {
    pub queue: &'static str,
    pub consumer_tag: String,
    pub prefetch: u16,
}

impl ConsumerConfig {
    /// 默认配置：prefetch 10（见 `DEFAULT_PREFETCH`），tag = `worker-{queue}-pid`。
    pub fn for_queue(queue: &'static str) -> Self {
        Self {
            queue,
            consumer_tag: format!("worker-{queue}-{}", std::process::id()),
            prefetch: crate::mq::DEFAULT_PREFETCH,
        }
    }
}

/// 单个 worker 的 future 类型。
pub type Consumer = Pin<Box<dyn Future<Output = ()> + Send>>;

/// 启动一个 consumer：
///   1. 建 connection（独立 connection，非 pool）
///   2. create channel + basic_qos
///   3. basic_consume 拿 stream
///   4. 每条 delivery → handler → ack/nack
///   5. 出错 → backoff 重连（外层 loop）
pub fn spawn_consumer(
    amqp_url: String,
    config: ConsumerConfig,
    handler: Handler,
) -> Consumer {
    Box::pin(async move {
        let mut backoff = Duration::from_secs(1);
        loop {
            match run_one_session(&amqp_url, &config, &handler).await {
                Ok(()) => {
                    tracing::info!(
                        "Consumer {} exited cleanly, reconnecting in {backoff:?}",
                        config.queue
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Consumer {} session error: {e}; reconnecting in {backoff:?}",
                        config.queue
                    );
                }
            }
            tokio::time::sleep(backoff).await;
            // 指数退避，cap 在 MAX_RECONNECT_BACKOFF
            backoff = (backoff * 2).min(MAX_RECONNECT_BACKOFF);
        }
    })
}

async fn run_one_session(
    amqp_url: &str,
    config: &ConsumerConfig,
    handler: &Handler,
) -> Result<(), String> {
    let conn = Connection::connect(amqp_url, ConnectionProperties::default())
        .await
        .map_err(|e| format!("connect: {e}"))?;
    let channel = conn
        .create_channel()
        .await
        .map_err(|e| format!("create_channel: {e}"))?;
    channel
        .basic_qos(config.prefetch, BasicQosOptions::default())
        .await
        .map_err(|e| format!("basic_qos: {e}"))?;

    let mut consumer = channel
        .basic_consume(
            config.queue,
            &config.consumer_tag,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .map_err(|e| format!("basic_consume: {e}"))?;

    tracing::info!("Consumer started: queue={}, tag={}", config.queue, config.consumer_tag);

    while let Some(delivery) = consumer.next().await {
        let delivery = match delivery {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Consumer {} delivery error: {e}", config.queue);
                continue;
            }
        };

        let bytes = delivery.data.clone();
        let outcome = (handler)(bytes).await;

        match outcome {
            HandlerOutcome::Ack => {
                if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                    tracing::warn!("ack failed: {e}");
                }
                metrics::MQ_CONSUME_TOTAL
                    .with_label_values(&[config.queue, "ack"])
                    .inc();
            }
            HandlerOutcome::Retry => {
                if let Err(e) = delivery
                    .nack(BasicNackOptions {
                        multiple: false,
                        requeue: true,
                    })
                    .await
                {
                    tracing::warn!("nack(requeue) failed: {e}");
                }
                metrics::MQ_CONSUME_TOTAL
                    .with_label_values(&[config.queue, "retry"])
                    .inc();
            }
            HandlerOutcome::Drop => {
                if let Err(e) = delivery
                    .nack(BasicNackOptions {
                        multiple: false,
                        requeue: false,
                    })
                    .await
                {
                    tracing::warn!("nack(drop) failed: {e}");
                }
                metrics::MQ_CONSUME_TOTAL
                    .with_label_values(&[config.queue, "drop"])
                    .inc();
            }
        }
    }

    Err("consumer stream ended (channel closed)".to_string())
}

/// 内部 helper：用于在测试 / 单元测试里 mock 一个 channel。
#[cfg(test)]
pub async fn ack_test_only(channel: &Channel, delivery_tag: u64) -> Result<(), String> {
    use lapin::options::BasicAckOptions;
    channel
        .basic_ack(delivery_tag, BasicAckOptions::default())
        .await
        .map_err(|e| e.to_string())
}
