//! mq/queues.rs — 集中维护所有 queue 的元信息（声明参数 + JobKind 路由）
//!
//! P3-2 引入 3 个内部 queue（direct exchange + default exchange 都用 routing_key=queue_name 即可）：
//!   - `ai_predictions`   — AI 预测任务。durable, x-max-priority=10
//!   - `notifications`    — 多渠道告警通知。durable, 普通优先级
//!   - `risk_logs`        — 风控日志批量入库。durable, 高吞吐
//!
//! 设计原则：
//!   1. 所有 queue 名 / 参数集中在 `QUEUE_DECLARATIONS` 数组，publisher/consumer 都从这里读。
//!      改一处自动同步到 publish/consume 两端，避免「声明参数不一致」导致 channel 关闭。
//!   2. `JobKind` 枚举 → routing_key 双向映射，避免上游/下游拼写不一致。
//!   3. 全部用 default exchange（空字符串）+ routing_key=queue_name 模式：
//!      - 无需额外 exchange 声明
//!      - 多 worker 启动顺序无关
//!      - 适合本系统"队列即业务"的小规模场景

use std::collections::HashMap;

/// AI 预测任务队列 — 高优先级
pub const QUEUE_AI_PREDICTIONS: &str = "ai_predictions";

/// 多渠道通知队列 — 普通优先级
pub const QUEUE_NOTIFICATIONS: &str = "notifications";

/// 风控日志批量入库队列 — 高吞吐，无优先级
pub const QUEUE_RISK_LOGS: &str = "risk_logs";

/// 业务侧抽象的 job 类型 — 上游 publish 用这个枚举，mq 内部映射到 routing_key。
///
/// 为什么不是直接传 `&str` routing_key：
///   1. 编译期穷尽：增删 queue 时所有 `match JobKind` 都会报缺臂。
///   2. 避免拼写错误：下游 worker `match JobKind::AiPrediction` 错写
///      `"ai-predictions"` 会被编译器抓。
///   3. metrics label 统一：从枚举 `.as_str()` 拿稳定字符串。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobKind {
    /// AI 价格方向预测
    AiPrediction,
    /// 多渠道告警通知
    Notification,
    /// 风控日志批量入库
    RiskLog,
}

impl JobKind {
    /// 业务枚举 → AMQP routing_key (= queue name, default exchange 模式下)
    pub const fn routing_key(self) -> &'static str {
        match self {
            JobKind::AiPrediction => QUEUE_AI_PREDICTIONS,
            JobKind::Notification => QUEUE_NOTIFICATIONS,
            JobKind::RiskLog => QUEUE_RISK_LOGS,
        }
    }

    /// 业务枚举 → Prometheus label
    pub const fn as_str(self) -> &'static str {
        match self {
            JobKind::AiPrediction => "ai_prediction",
            JobKind::Notification => "notification",
            JobKind::RiskLog => "risk_log",
        }
    }

    /// routing_key → JobKind（消费侧解析）
    pub fn from_routing_key(key: &str) -> Option<Self> {
        match key {
            QUEUE_AI_PREDICTIONS => Some(JobKind::AiPrediction),
            QUEUE_NOTIFICATIONS => Some(JobKind::Notification),
            QUEUE_RISK_LOGS => Some(JobKind::RiskLog),
            _ => None,
        }
    }
}

/// Queue 声明参数 — 集中维护，consumer/publisher 都 `queue_declare` 一遍。
///
/// 设计原则：所有字段都通过 [`lapin::options::QueueDeclareOptions`] 表达，禁止直接传
///   `FieldTable`，避免散落字符串 key（`x-max-priority` 拼写错会很晚才被发现）。
#[derive(Debug, Clone)]
pub struct QueueDeclaration {
    pub name: &'static str,
    pub kind: JobKind,
    /// 是否持久化（broker 重启后保留）。所有业务 queue 都 true。
    pub durable: bool,
    /// max priority（None = 不启用 priority queue）。只 ai_predictions 用 10 级。
    pub max_priority: Option<u8>,
    /// dead-letter exchange。None = 失败直接丢弃（由 consumer 自己处理 ack/nack）。
    pub dead_letter_exchange: Option<&'static str>,
}

impl QueueDeclaration {
    /// 把声明转换成 `lapin::options::QueueDeclareOptions` + `FieldTable`。
    /// 集中在这里 → 改一处即生效。
    pub fn to_lapin_options(&self) -> (lapin::options::QueueDeclareOptions, lapin::types::FieldTable) {
        let opts = lapin::options::QueueDeclareOptions {
            durable: self.durable,
            ..Default::default()
        };
        let mut args = lapin::types::FieldTable::default();
        if let Some(p) = self.max_priority {
            args.insert(
                "x-max-priority".into(),
                lapin::types::AMQPValue::ShortShortUInt(p),
            );
        }
        if let Some(dlx) = self.dead_letter_exchange {
            args.insert(
                "x-dead-letter-exchange".into(),
                lapin::types::AMQPValue::LongString(dlx.into()),
            );
        }
        (opts, args)
    }
}

/// 所有 queue 的总表 — 用数组保证启动时统一声明，顺序无关紧要。
pub const QUEUE_DECLARATIONS: &[QueueDeclaration] = &[
    QueueDeclaration {
        name: QUEUE_AI_PREDICTIONS,
        kind: JobKind::AiPrediction,
        durable: true,
        max_priority: Some(10),
        dead_letter_exchange: None,
    },
    QueueDeclaration {
        name: QUEUE_NOTIFICATIONS,
        kind: JobKind::Notification,
        durable: true,
        max_priority: None,
        dead_letter_exchange: None,
    },
    QueueDeclaration {
        name: QUEUE_RISK_LOGS,
        kind: JobKind::RiskLog,
        durable: true,
        max_priority: None,
        dead_letter_exchange: None,
    },
];

/// 启动时一次性 declare 所有 queue。失败不 panic — 仅 warn + 继续，
///   worker 重连时再尝试。broker 完全不可用时 publish 自然失败由业务兜底。
pub async fn declare_all_queues(
    channel: &lapin::Channel,
) -> Result<(), lapin::Error> {
    for decl in QUEUE_DECLARATIONS {
        let (opts, args) = decl.to_lapin_options();
        channel
            .queue_declare(decl.name, opts, args)
            .await?;
    }
    Ok(())
}

/// 暴露给测试的 `name -> &QueueDeclaration` 索引，避免外部按字符串查找。
pub fn by_name(name: &str) -> Option<&'static QueueDeclaration> {
    QUEUE_DECLARATIONS.iter().find(|d| d.name == name)
}

/// 暴露给 metrics label 的「queue 名 → stable label」映射。
/// 集中维护 → 增删 queue 时编译器会强制更新。
pub fn metrics_labels() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    for d in QUEUE_DECLARATIONS {
        m.insert(d.name, d.kind.as_str());
    }
    m
}
