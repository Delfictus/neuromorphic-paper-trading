//! Exchange error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExchangeError {
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    
    #[error("Parsing error: {0}")]
    Parse(String),
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),
    
    #[error("Order rejected: {0}")]
    OrderRejected(String),
    
    #[error("Insufficient balance")]
    InsufficientBalance,
    
    #[error("Network timeout")]
    Timeout,
    
    #[error("Sequence gap detected: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Recoverable,
    Fatal,
    RateLimit,
}

impl ExchangeError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Connection(_) | Self::WebSocket(_) | Self::Timeout => ErrorKind::Recoverable,
            Self::RateLimit => ErrorKind::RateLimit,
            Self::AuthenticationFailed | Self::InsufficientBalance => ErrorKind::Fatal,
            _ => ErrorKind::Recoverable,
        }
    }
    
    pub fn should_retry(&self) -> bool {
        matches!(self.kind(), ErrorKind::Recoverable)
    }
}

impl From<serde_json::Error> for ExchangeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse(err.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for ExchangeError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(err.to_string())
    }
}

impl From<reqwest::Error> for ExchangeError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout
        } else if err.is_connect() {
            Self::Connection(err.to_string())
        } else {
            Self::Unknown(err.to_string())
        }
    }
}