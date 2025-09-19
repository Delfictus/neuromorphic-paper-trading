//! Exchange connectivity modules

pub mod binance;
pub mod types;
pub mod errors;
pub mod orderbook;
pub mod book_manager;
pub mod connector;
pub mod websocket;
pub mod binance_websocket;

pub use binance::{BinanceWebSocket, MultiSymbolTracker};
pub use types::{
    MarketDataType, ExchangeMessage, Symbol, Side, OrderType, Exchange, TimeInForce,
    UniversalTrade, UniversalQuote, UniversalOrderBook, UniversalMarketData,
};
pub use errors::{ExchangeError as LegacyExchangeError, ErrorKind};
pub use orderbook::{OrderBook, DepthUpdate};
pub use book_manager::{OrderBookManager, ArbitrageOpportunity};

// Re-export the new comprehensive connector interface
pub use connector::{
    ExchangeConnector, ExchangeResult, ExchangeError,
    UniversalOrder, OrderRequest, TradeExecution, TradeFee,
    Balance, AccountInfo, AccountType, Permission, OrderStatus,
    UniversalTicker, UniversalKline, KlineInterval,
    ExchangeInfo, SymbolInfo, SymbolStatus, RateLimit, RateLimitType, RateLimitInterval,
};

// Re-export WebSocket streaming interface
pub use websocket::{
    StreamManager, WebSocketManager, WebSocketConfig,
    StreamType, StreamSubscription, ConnectionStatus, StreamMetrics,
};

// Re-export Binance WebSocket implementation
pub use binance_websocket::BinanceWebSocketManager;

use async_trait::async_trait;
use anyhow::Result;

/// Legacy trait for exchange connectors (WebSocket-based market data)
/// This is kept for backward compatibility with existing WebSocket implementations
#[async_trait]
pub trait LegacyExchangeConnector: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn subscribe(&mut self, symbols: Vec<&str>) -> Result<()>;
    fn try_recv(&mut self) -> Option<UniversalMarketData>;
    async fn start(&mut self) -> Result<()>;
    fn name(&self) -> &str;
}