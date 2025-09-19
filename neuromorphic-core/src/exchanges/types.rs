//! Exchange data types

use serde::{Deserialize, Serialize};
use std::fmt;

/// Market data types
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MarketDataType {
    Trade,
    Quote,
    OrderBook,
    Kline,
}

/// Exchange message wrapper
#[derive(Clone, Debug)]
pub struct ExchangeMessage {
    pub timestamp: u64,
    pub symbol: Symbol,
    pub data: MarketDataType,
    pub exchange: Exchange,
    pub sequence: u64,
}

/// Trading symbol
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol(pub String);

impl Symbol {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    
    pub fn validate(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Order side
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn multiplier(&self) -> f64 {
        match self {
            Side::Buy => 1.0,
            Side::Sell => -1.0,
        }
    }
}

/// Order types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit { price: f64 },
    StopLimit { stop: f64, limit: f64 },
}

/// Exchange identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    Binance,
    Coinbase,
    Kraken,
    Bitstamp,
    Gemini,
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Exchange::Binance => write!(f, "Binance"),
            Exchange::Coinbase => write!(f, "Coinbase"),
            Exchange::Kraken => write!(f, "Kraken"),
            Exchange::Bitstamp => write!(f, "Bitstamp"),
            Exchange::Gemini => write!(f, "Gemini"),
        }
    }
}

/// Universal trade format
#[derive(Clone, Debug)]
pub struct UniversalTrade {
    pub exchange: Exchange,
    pub symbol: Symbol,
    pub price: f64,
    pub quantity: f64,
    pub side: Side,
    pub timestamp_exchange: u64,
    pub timestamp_local: u64,
    pub trade_id: String,
}

/// Universal quote format
#[derive(Clone, Debug)]
pub struct UniversalQuote {
    pub exchange: Exchange,
    pub symbol: Symbol,
    pub bid_price: f64,
    pub bid_size: f64,
    pub ask_price: f64,
    pub ask_size: f64,
    pub timestamp_exchange: u64,
    pub timestamp_local: u64,
}

/// Universal order book format
#[derive(Clone, Debug)]
pub struct UniversalOrderBook {
    pub exchange: Exchange,
    pub symbol: Symbol,
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
    pub timestamp_exchange: u64,
    pub timestamp_local: u64,
    pub sequence: u64,
}

/// Universal market data enum
#[derive(Clone, Debug)]
pub enum UniversalMarketData {
    Trade(UniversalTrade),
    Quote(UniversalQuote),
    OrderBook(UniversalOrderBook),
}

impl UniversalMarketData {
    pub fn timestamp(&self) -> u64 {
        match self {
            Self::Trade(t) => t.timestamp_local,
            Self::Quote(q) => q.timestamp_local,
            Self::OrderBook(b) => b.timestamp_local,
        }
    }
    
    pub fn symbol(&self) -> &Symbol {
        match self {
            Self::Trade(t) => &t.symbol,
            Self::Quote(q) => &q.symbol,
            Self::OrderBook(b) => &b.symbol,
        }
    }
}

/// Time in force options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TimeInForce {
    GTC,  // Good Till Cancel
    IOC,  // Immediate or Cancel
    FOK,  // Fill or Kill
    GTX,  // Good Till Extended
}