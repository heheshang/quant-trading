//! Binance Exchange Connector Error Types

use thiserror::Error;

/// Connector-specific errors for exchange connections.
#[derive(Error, Debug)]
pub enum ConnectorError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("Message parse error: {0}")]
    MessageParseError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Rate limited: retry after {0}s")]
    RateLimited(u64),

    #[error("Symbol not supported: {0}")]
    SymbolNotSupported(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Disconnected: {0}")]
    Disconnected(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl ConnectorError {
    /// Returns true if this error indicates a transient failure that should retry.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ConnectorError::ConnectionFailed(_)
                | ConnectorError::RateLimited(_)
                | ConnectorError::Timeout(_)
                | ConnectorError::Disconnected(_)
        )
    }

    /// Returns the recommended retry delay in seconds.
    pub fn retry_delay(&self) -> u64 {
        match self {
            ConnectorError::RateLimited(s) => *s,
            ConnectorError::ConnectionFailed(_) => 1,
            ConnectorError::Disconnected(_) => 1,
            ConnectorError::Timeout(_) => 5,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retryable_errors() {
        let err = ConnectorError::ConnectionFailed("network".to_string());
        assert!(err.is_retryable());

        let err = ConnectorError::SymbolNotSupported("INVALID".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_retry_delay() {
        let err = ConnectorError::RateLimited(60);
        assert_eq!(err.retry_delay(), 60);

        let err = ConnectorError::ConnectionFailed("timeout".to_string());
        assert_eq!(err.retry_delay(), 1);
    }
}