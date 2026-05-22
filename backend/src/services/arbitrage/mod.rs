//! 套利模块 services
//!
//! 子模块：
//! - types.rs           : 数据类型（套利对、持仓、信号、SpreadWindow）
//! - spread_calculator.rs : 价差计算引擎（ratio/percentage/zscore）
//! - signal_generator.rs  : 信号生成器（EntryLong/EntryShort/Exit/StopLoss）

pub mod signal_generator;
pub mod spread_calculator;
pub mod types;

pub use signal_generator::*;
pub use spread_calculator::*;
pub use types::*;
