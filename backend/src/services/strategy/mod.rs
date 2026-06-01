//! 策略服务
//!
//! 按职责拆分为 4 个子模块：
//! - [`common`]            — `StrategyTemplate` trait + 内部 indicator helpers
//! - [`templates_impls`]   — 10 个内置策略模板实现
//! - [`registry`]          — 模板注册表 + 状态校验
//! - [`crud`]              — CRUD、导入/导出、批量操作、代码存储
//!
//! 所有 public API 都通过本模块重新导出，调用方不需要关心子模块划分。
//! `TemplateInfo` 来自 `crate::models::schemas`（非本模块定义），保持原引用。

#[macro_use]
mod common;
mod crud;
mod registry;
mod templates_impls;

pub use common::StrategyTemplate;
pub use crud::{
    bulk_delete_strategies, bulk_update_status, create_strategy, delete_strategy,
    export_strategies, get_strategy, import_batch, import_strategy, list_strategies,
    list_templates, store_strategy_code, update_strategy, update_strategy_status, BulkDeleteResponse,
};
pub use registry::{get_all_templates, get_template};
