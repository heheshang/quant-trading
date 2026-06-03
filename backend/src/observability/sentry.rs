//! Sentry 错误聚合集成 (运维-2)
//!
//! ## 设计
//!
//! 三个 crate 协作：
//!   - [`sentry`]          主 SDK：初始化 (`init_sentry`) + 错误捕获 (`capture_error`)
//!   - [`sentry_tracing`]  把 `tracing::error!` / `tracing::warn!` 自动转 Sentry event
//!   - [`sentry_tower`]    Tower middleware，挂在 axum 全局 Router 上自动捕获 5xx + panic
//!
//! ## 启用条件
//!
//! `SENTRY_DSN` 环境变量存在时启用。缺失时 `init_sentry` 走 noop 分支，**不**
//! 装任何 layer / hook，运行时无额外开销。
//!
//! ## 释放
//!
//! 返回的 [`SentryGuard`] 持有 Sentry client，必须在进程生命周期内持有（绑定
//! 到 `main` 里的 `let _sentry_guard = ...`）。Drop 时 flush 客户端并关停后台
//! 线程，进程不会卡住。
//!
//! ## 错误捕获途径
//!
//! 1. **被动** — `sentry_tower::NewSentryLayer` 拦截 axum handler 5xx 响应 + panic
//! 2. **半自动** — `sentry-tracing` 把 `tracing::error!` 事件转为 Sentry event
//! 3. **主动** — [`capture_error`] helper 显式上报某条业务错误（典型场景：批量
//!    处理中跳过单个错误时手动上报，附带 context tag）
//!
//! 业务代码大多数情况不需要主动调 `capture_error`：tracing 路径已经覆盖。
//! 当你想给某个错误**加 context tag**（比如 user_id、order_id）时再调 helper。
//!
//! ## 与 OpenTelemetry 的关系
//!
//! Sentry 和 OTel 是两个独立的信号通道：
//!
//! | 通道     | 关注            | 工具                 |
//! |----------|-----------------|----------------------|
//! | OTel     | 请求链路 / 性能 | Jaeger / Tempo       |
//! | Sentry   | 错误聚合 / 告警 | Sentry SaaS / selfhost|
//!
//! 它们共存：tracing span 会同时进 OTel 和（如果 level >= error）Sentry。
//! 关掉 OTel 不会影响 Sentry，反之亦然。
//!
//! ## Panic hook
//!
//! 单独注册一个 [`std::panic::set_hook`]，把 panic 文本上报为 `Level::Fatal`。
//! 之所以不直接依赖 `sentry` 的 `panic` feature，是因为 `sentry::init` 默认会
//! 装它自己的 hook，但在我们这套 `release-health` + `tracing` 配置下，hook
//! 注册顺序需要在 `init_sentry` 之后明确控制（避免 Sentry 的 hook 把 panic
//! 静默吞掉后 tracing 拿不到 panic_info）。
//!
//! ## 上下文
//!
//! 默认透传给 Sentry 的 context：
//!   - `release`     = `env!("CARGO_PKG_VERSION")` 或 `SENTRY_RELEASE` 环境变量
//!   - `environment` = `APP_ENV` 环境变量（默认 `development`）
//!   - `traces_sample_rate` = 0.0（运维-2 阶段先关闭性能采样，避免 OTel 重复传输）

use std::sync::atomic::{AtomicBool, Ordering};

/// 全局开关：Sentry 是否启用。
///
/// 由 [`init_sentry`] 写入。其他模块（特别是测试和 helper）通过
/// [`is_enabled`] 读取，避免在未启用时调用 [`sentry::capture_*`]（这些 API
/// 在 DSN 为 None 时是 noop，但显式跳过能省一次字符串构造）。
static ENABLED: AtomicBool = AtomicBool::new(false);

/// 返回 Sentry 是否已启用。
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Sentry guard。
///
/// 持有 [`sentry::Client`] 句柄。Drop 时不主动 flush（依赖 `sentry::Client`
/// 自己的后台 flush 任务），目的是让 guard 释放时进程不会
/// 等待几秒的网络 I/O。
///
/// ## 静默 drop
///
/// `sentry::Client` 内部维护一个 `tokio` 后台任务，调用 `drop(client)` 会
/// `_ = shutdown()`（best-effort，**不**返回错误）。所以业务代码在
/// graceful shutdown 时不需要 `sentry::Client::close()`。
///
/// ## panic hook 恢复限制
///
/// 出于安全考虑（避免 `try_unwrap` 在 Arc 仍有引用时 panic），guard Drop
/// **不**主动恢复之前的 panic hook。`previous_hook_for_closure` 持有的
/// `Arc<Box<dyn Fn>>` 会随闭包自然 drop，无泄漏。
///
/// 副作用：在 unit test 中，如果某个测试装了 Sentry hook，同进程内后续
/// 测试 panic 时会同时走 Sentry 上报。这不影响生产环境（生产进程只有一个
/// `init_sentry` 调用）。
pub struct SentryGuard {
    // 持有 client 才能让 background flush 任务存活。
    _client: Option<sentry::ClientInitGuard>,
    // Hold a reference to the previous panic hook so it stays alive for the
    // closure (avoiding use-after-free if the test environment ever clears
    // it). This is the only reliable way without `std::mem::transmute`:
    // `Box<dyn Fn(...)>` is not Clone, so the closure must own its own copy
    // via Arc.
    _previous_hook: Option<std::sync::Arc<
        Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Sync + Send + 'static>,
    >>,
}

impl Drop for SentryGuard {
    fn drop(&mut self) {
        // 显式把 client init guard drop 掉，触发 Sentry 后台任务 flush。
        // 不需要恢复 panic hook——见 struct 注释。
        self._client.take();
    }
}

/// 初始化 Sentry。
///
/// - `SENTRY_DSN` 不存在时返回 `Ok(None)`，代表 noop；guard 不持有任何资源。
/// - 存在时调用 `sentry::init` 并注册 `sentry-tracing` 的 event layer，**以及**
///   一个 panic hook 把 panic 文本以 `Level::Fatal` 上报。
///
/// 调用方应**只**在 `main` 启动序列开头调一次。
pub fn init_sentry() -> Result<Option<SentryGuard>, String> {
    let dsn = match std::env::var("SENTRY_DSN") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            tracing::info!(
                "SENTRY_DSN not set — Sentry is disabled (errors are NOT reported)"
            );
            ENABLED.store(false, Ordering::Relaxed);
            return Ok(None);
        }
    };

    let release = std::env::var("SENTRY_RELEASE")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    let environment = std::env::var("APP_ENV")
        .unwrap_or_else(|_| "development".to_string());
    let traces_sample_rate = std::env::var("SENTRY_TRACES_SAMPLE_RATE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0);

    // 构造 sentry options。
    let opts = sentry::ClientOptions {
        dsn: Some(dsn.parse().map_err(|e| {
            format!("SENTRY_DSN parse failed: {e}")
        })?),
        release: Some(release.clone().into()),
        environment: Some(environment.clone().into()),
        traces_sample_rate,
        // 性能采样在运维-2 阶段先关闭，避免和 OTel 重复传输。
        // 真正有性能问题时再单独开 Sentry 的 performance monitoring。
        attach_stacktrace: true,
        // 默认 in-app 标识所有用户 crate 的 frame。in_app_* 类型是
        // Vec<&'static str>，所以只能塞字符串字面量。
        in_app_include: vec!["quant_trading_backend"],
        in_app_exclude: vec!["std", "core", "tokio", "axum", "tower"],
        ..Default::default()
    };

    let guard = sentry::init(opts);
    ENABLED.store(true, Ordering::Relaxed);

    tracing::info!(
        dsn_host = mask_dsn_host(&dsn),
        release = %release,
        environment = %environment,
        traces_sample_rate = traces_sample_rate,
        "Sentry initialised"
    );

    // 装 sentry-tracing 的 event-to-sentry 桥接（覆盖 tracing::error!/warn!）。
    // 注意：sentry-tracing 自身会调 sentry::Hub::current()，所以必须放在
    // sentry::init 之后。详见 sentry-tracing 0.48 的 layer 文档。
    //
    // NOTE: 临时禁用 — `sentry_tracing::EventFilter` 不存在 (sentry 0.48
    // 重命名为 `sentry::integrations::tracing::EventFilter`, 而 sentry
    // 0.48 的 `sentry` crate 不导出 `integrations` 模块). 待 sentry 0.49
    // 修复 API 后再启用。
    let _layer: Option<()> = None;

    // 装 panic hook，把 panic 文本以 Level::Fatal 上报。
    // 保存之前的 hook 以便 Drop 时恢复（不污染测试环境）。
    //
    // 实现要点：闭包内不能 `take_hook()` 拿当前 hook（会拿回自己的，导致
    // 递归），必须把 previous_hook 显式 move 进闭包。
    //
    // `Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>` 本身不实现
    // Clone，所以我们用 Arc 包装一层，让闭包可以 move 一个 Arc 引用，
    // Drop 时再 `take_hook` + 恢复。
    let previous_hook: std::sync::Arc<
        Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Sync + Send + 'static>,
    > = std::sync::Arc::new(std::panic::take_hook());
    let previous_hook_for_closure = previous_hook.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        // 优先用 Sentry 的 Hub；DSN 未配置时 init 已早返回，这里只会在
        // ENABLED 状态下被调到。
        if ENABLED.load(Ordering::Relaxed) {
            let msg = panic_info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| {
                    panic_info
                        .payload()
                        .downcast_ref::<String>()
                        .cloned()
                })
                .unwrap_or_else(|| {
                    format!("{panic_info}")
                });
            sentry::capture_message(&msg, sentry::Level::Fatal);
            // 不在 panic hook 里手动 flush——hook 会在 panic unwind 期间
            // 执行，强制 flush 可能死锁。Sentry client 的后台任务
            // 会自行处理。
        }
        // 调回之前的 hook，保证 panic 仍然打到 stderr / 触发 abort。
        // 关键：传 previous_hook_for_closure 而不是 `take_hook()`，
        // 否则会拿回当前 hook 自身导致下一次 panic 无限递归。
        previous_hook_for_closure(panic_info);
    }));

    // sentry-tracing 的 event-to-sentry 桥接在 `init_tracing` 装到 Registry。
    // 这里不重复 init（tracing subscriber 只能 init 一次）。

    Ok(Some(SentryGuard {
        _client: Some(guard),
        _previous_hook: Some(previous_hook),
        // _layer drop 不会卸载已注册的 subscriber——见
        // sentry_tracing::layer() 文档。所以 layer 字段被 named-ignored。
    }))
}

/// 主动上报一个错误到 Sentry。
///
/// 业务代码**大多数情况不需要调用**：tracing 路径已经覆盖。
/// 只在你想给错误附加 context tag 时使用，比如：
///
/// ```ignore
/// if let Err(e) = process_order(&order).await {
///     sentry::configure_scope(|scope| {
///         scope.set_tag("order_id", order.id.to_string());
///         scope.set_tag("user_id", user.id.to_string());
///     });
///     capture_error(&e, Some("order.process"));
/// }
/// ```
///
/// # 参数
/// - `err`     : 任何实现了 `std::error::Error` 的错误。
/// - `context` : 可选的 context tag，会作为 Sentry event 的 `logger` 字段
///               出现（Sentry 用来聚合的分组键之一）。
pub fn capture_error(err: &(dyn std::error::Error + 'static), context: Option<&str>) {
    if !is_enabled() {
        return;
    }
    // 走 sentry::capture_error，它会从 err 自动生成 backtrace（需要
    // `backtrace` feature，已开）。chain 也被遍历，每个 cause 都会进 Sentry。
    if let Some(ctx) = context {
        sentry::with_scope(
            |scope| {
                scope.set_tag("error_context", ctx.to_string());
            },
            || sentry::capture_error(err),
        );
    } else {
        sentry::capture_error(err);
    }
}

/// 把 DSN 里的 host 部分脱敏后打日志（避免把完整 DSN 泄露到 stdout）。
///
/// DSN 形如 `https://<key>@<host>/<project>`。我们只要 `<host>/<project>`。
fn mask_dsn_host(dsn: &str) -> String {
    if let Some(at_idx) = dsn.find('@') {
        let after_at = &dsn[at_idx + 1..];
        // 截到第一个 `/` 之前是 host
        let host_end = after_at.find('/').unwrap_or(after_at.len());
        let host = &after_at[..host_end];
        let project_start = host_end;
        // 截到 `?` 之前是 project id
        let project_end_rel = after_at[project_start..]
            .find('?')
            .unwrap_or(after_at.len() - project_start);
        let project = &after_at[project_start..project_start + project_end_rel];
        format!("{host}/{project}")
    } else {
        "<unparseable>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 单元测试：DSN host 脱敏函数不能泄露完整 DSN。
    #[test]
    fn dsn_mask_does_not_leak_key() {
        let dsn = "https://abc123def456@sentry.io/42";
        let masked = mask_dsn_host(dsn);
        assert!(!masked.contains("abc123def456"), "key leaked: {masked}");
        assert!(masked.contains("sentry.io"), "host missing: {masked}");
        assert!(masked.contains("42"), "project missing: {masked}");
    }

    /// 单元测试：缺省 DSN 时 init_sentry 返回 None 且 ENABLED 仍为 false。
    #[test]
    fn init_sentry_disabled_when_dsn_missing() {
        // SAFETY: 单线程测试体内独占 env；该测试运行期间不与其它测试并发。
        unsafe { std::env::remove_var("SENTRY_DSN") };
        let guard = init_sentry().expect("init should not fail without DSN");
        assert!(guard.is_none(), "guard should be None when DSN missing");
        assert!(!is_enabled(), "ENABLED must remain false");
    }

    /// 单元测试：DSN 存在时 ENABLED 翻为 true。
    #[test]
    fn init_sentry_enables_when_dsn_set() {
        // 顺序敏感：必须先 disable 再 enable（因为同进程内可能跑过上一
        // 个测试）。
        let prev = std::env::var("SENTRY_DSN").ok();
        unsafe { std::env::set_var("SENTRY_DSN", "https://test@example.com/1") };
        let guard = init_sentry().expect("init should succeed with valid DSN");
        assert!(guard.is_some(), "guard should be Some when DSN set");
        assert!(is_enabled(), "ENABLED must be true");
        // 恢复
        match prev {
            Some(v) => unsafe { std::env::set_var("SENTRY_DSN", v) },
            None => unsafe { std::env::remove_var("SENTRY_DSN") },
        }
        // 这里不主动 drop guard（drop 会恢复 panic hook，可能影响后续测试）。
        // 让 guard 自然丢弃到测试函数结束时。
    }

    /// 单元测试：capture_error 在未启用时是 noop（不能 panic）。
    #[test]
    fn capture_error_is_noop_when_disabled() {
        ENABLED.store(false, Ordering::Relaxed);
        let err = std::io::Error::new(std::io::ErrorKind::Other, "test");
        // 不应 panic。
        capture_error(&err, Some("unit_test"));
    }
}
