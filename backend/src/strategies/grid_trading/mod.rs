//! Grid Trading Module — P3-F2
//!
//! Grid trading with optional Martingale position sizing.
//!
//! # Architecture
//! - `types.rs`: Core types (GridConfig, GridOrder, GridStatus, MarketMode)
//! - `market_mode.rs`: Trend/Ranging market detection (ADX + Bollinger Bandwidth)
//! - `martingale.rs`: Martingale position sizing tracker
//! - `grid_engine.rs`: Core grid engine implementation

pub mod grid_engine;
pub mod market_mode;
pub mod martingale;
pub mod types;

pub use grid_engine::GridEngine;
pub use market_mode::MarketModeDetector;
pub use martingale::MartingaleTracker;
pub use types::*;
