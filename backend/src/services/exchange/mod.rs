//! Exchange Connector Module
//!
//! Provides a unified interface for connecting to cryptocurrency exchanges.
//! Currently implements Binance WebSocket connector.

pub mod api_keys;
pub mod binance_connector;
pub mod errors;
pub mod rate_limiter;
pub mod signed_client;
pub mod types;
pub mod ws_hub;

pub use api_keys::ApiKeyStore;
pub use binance_connector::BinanceConnector;
pub use errors::ConnectorError;
pub use rate_limiter::RateLimiter;
pub use signed_client::{
    AccountInfo, Balance, CancelOrderResponse, Fill, NewOrder, OrderResponse, PingResponse,
    RateLimitInfo, SignedBinanceClient,
};
pub use types::{MarketMessage, SUPPORTED_SYMBOLS};
pub use ws_hub::{HubEvent, HubMessage, WsHub};
