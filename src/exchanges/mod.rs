//! Exchange connectivity modules

pub mod binance;
pub mod types;
pub mod errors;
pub mod orderbook;
pub mod book_manager;

pub use binance::{BinanceWebSocket, MultiSymbolTracker};
pub use types::{
    MarketDataType, ExchangeMessage, Symbol, Side, OrderType, Exchange,
    UniversalTrade, UniversalQuote, UniversalOrderBook, UniversalMarketData,
};
pub use errors::{ExchangeError, ErrorKind};
pub use orderbook::{OrderBook, DepthUpdate};
pub use book_manager::{OrderBookManager, ArbitrageOpportunity};

use async_trait::async_trait;
use anyhow::Result;

/// Trait for exchange connectors
#[async_trait]
pub trait ExchangeConnector: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn subscribe(&mut self, symbols: Vec<&str>) -> Result<()>;
    fn try_recv(&mut self) -> Option<UniversalMarketData>;
    async fn start(&mut self) -> Result<()>;
    fn name(&self) -> &str;
}