//! Exchange Connector Module
//!
//! Provides a unified interface for connecting to cryptocurrency exchanges.
//! Currently implements Binance WebSocket connector.

pub mod binance_connector;
pub mod errors;
pub mod types;
pub mod ws_hub;

pub use binance_connector::BinanceConnector;
pub use errors::ConnectorError;
pub use types::{MarketMessage, SUPPORTED_SYMBOLS};
pub use ws_hub::{HubEvent, HubMessage, WsHub};
