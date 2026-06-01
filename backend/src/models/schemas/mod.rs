//! 全部请求/响应 DTO（按领域分文件）
//!
//! 为保持向后兼容，所有公开类型在子模块中定义后通过 `pub use` 在本模块
//! 根目录重新导出，调用方仍可使用 `crate::models::schemas::Xxx` 路径访问。

pub mod auth;
pub mod indicator;
pub mod market;
pub mod response;
pub mod strategy;

// `TickerHistoryResponse` is defined in `models::market_schemas` (re-exported
// through `auth::*` because auth.rs does `pub use crate::models::market_schemas::*`)
// and also redefined in `response.rs`. To avoid an ambiguous glob re-export
// warning, we re-export it explicitly from the local `response` module and
// hide the `market_schemas` version at this level. Downstream callers use
// `crate::models::schemas::TickerHistoryResponse` (the local one) for the
// `TickerHistoryMeta`-shaped payload; the market_schemas alias remains
// available via the `market_schemas` module for legacy callers.
#[allow(ambiguous_glob_reexports)]
pub use auth::*;
pub use indicator::*;
pub use market::*;
pub use response::*;
pub use strategy::*;
