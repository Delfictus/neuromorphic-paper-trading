//! Exchange connector trait and core data structures

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use chrono::{DateTime, Utc};

use super::types::{Symbol, Exchange, Side, OrderType, TimeInForce, UniversalTrade, UniversalQuote, UniversalOrderBook, UniversalMarketData};

/// Custom result type for exchange operations
pub type ExchangeResult<T> = Result<T, ExchangeError>;

/// Comprehensive error types for exchange operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExchangeError {
    #[error("Network error: {message}")]
    Network { message: String },
    
    #[error("Authentication failed: {reason}")]
    Authentication { reason: String },
    
    #[error("API error {code}: {message}")]
    Api { code: i32, message: String },
    
    #[error("Rate limit exceeded: {retry_after:?}")]
    RateLimit { retry_after: Option<u64> },
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Sequence gap detected: expected {expected}, got {received}")]
    SequenceGap { expected: u64, received: u64 },
    
    #[error("Invalid request: {details}")]
    InvalidRequest { details: String },
    
    #[error("Order error: {reason}")]
    OrderError { reason: String },
    
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: f64, available: f64 },
    
    #[error("Symbol not found: {symbol}")]
    SymbolNotFound { symbol: String },
    
    #[error("Connection error: {message}")]
    Connection { message: String },
    
    #[error("Timeout after {seconds} seconds")]
    Timeout { seconds: u64 },
    
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl From<serde_json::Error> for ExchangeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse(err.to_string())
    }
}

impl From<anyhow::Error> for ExchangeError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal { message: err.to_string() }
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for ExchangeError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::Connection { message: err.to_string() }
    }
}

/// Order status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
    PendingCancel,
}

/// Universal order representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalOrder {
    pub id: String,
    pub client_order_id: Option<String>,
    pub symbol: Symbol,
    pub side: Side,
    pub order_type: OrderType,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub remaining_quantity: f64,
    pub price: Option<f64>,
    pub stop_price: Option<f64>,
    pub status: OrderStatus,
    pub time_in_force: TimeInForce,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub exchange: Exchange,
    pub fees: Option<TradeFee>,
    pub metadata: HashMap<String, String>,
}

impl UniversalOrder {
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::New | OrderStatus::PartiallyFilled | OrderStatus::PendingCancel)
    }
    
    pub fn is_filled(&self) -> bool {
        matches!(self.status, OrderStatus::Filled)
    }
    
    pub fn fill_percentage(&self) -> f64 {
        if self.quantity > 0.0 {
            (self.filled_quantity / self.quantity) * 100.0
        } else {
            0.0
        }
    }
}

/// Order request for placing new orders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub symbol: Symbol,
    pub side: Side,
    pub order_type: OrderType,
    pub quantity: f64,
    pub price: Option<f64>,
    pub stop_price: Option<f64>,
    pub time_in_force: TimeInForce,
    pub client_order_id: Option<String>,
    pub reduce_only: bool,
    pub post_only: bool,
}

impl OrderRequest {
    pub fn market_buy(symbol: Symbol, quantity: f64) -> Self {
        Self {
            symbol,
            side: Side::Buy,
            order_type: OrderType::Market,
            quantity,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::IOC,
            client_order_id: None,
            reduce_only: false,
            post_only: false,
        }
    }
    
    pub fn market_sell(symbol: Symbol, quantity: f64) -> Self {
        Self {
            symbol,
            side: Side::Sell,
            order_type: OrderType::Market,
            quantity,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::IOC,
            client_order_id: None,
            reduce_only: false,
            post_only: false,
        }
    }
    
    pub fn limit_buy(symbol: Symbol, quantity: f64, price: f64) -> Self {
        Self {
            symbol,
            side: Side::Buy,
            order_type: OrderType::Limit { price },
            quantity,
            price: Some(price),
            stop_price: None,
            time_in_force: TimeInForce::GTC,
            client_order_id: None,
            reduce_only: false,
            post_only: false,
        }
    }
    
    pub fn limit_sell(symbol: Symbol, quantity: f64, price: f64) -> Self {
        Self {
            symbol,
            side: Side::Sell,
            order_type: OrderType::Limit { price },
            quantity,
            price: Some(price),
            stop_price: None,
            time_in_force: TimeInForce::GTC,
            client_order_id: None,
            reduce_only: false,
            post_only: false,
        }
    }
}

/// Trade execution details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    pub id: String,
    pub order_id: String,
    pub symbol: Symbol,
    pub side: Side,
    pub quantity: f64,
    pub price: f64,
    pub fee: TradeFee,
    pub timestamp: DateTime<Utc>,
    pub is_maker: bool,
}

/// Trading fee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeFee {
    pub asset: String,
    pub amount: f64,
    pub rate: f64,
}

/// Account balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
    pub total: f64,
}

impl Balance {
    pub fn new(asset: String, free: f64, locked: f64) -> Self {
        Self {
            asset,
            free,
            locked,
            total: free + locked,
        }
    }
    
    pub fn available_for_trading(&self) -> f64 {
        self.free
    }
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub account_id: String,
    pub account_type: AccountType,
    pub permissions: Vec<Permission>,
    pub can_trade: bool,
    pub can_withdraw: bool,
    pub can_deposit: bool,
    pub trading_fee_maker: f64,
    pub trading_fee_taker: f64,
    pub updated_at: DateTime<Utc>,
}

/// Account type enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccountType {
    Spot,
    Margin,
    Futures,
    Options,
}

/// Trading permissions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    Spot,
    Margin,
    Futures,
    Options,
    Leveraged,
    TradingBot,
}

/// Universal ticker data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalTicker {
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub price: f64,
    pub price_change: f64,
    pub price_change_percent: f64,
    pub high_24h: f64,
    pub low_24h: f64,
    pub volume_24h: f64,
    pub volume_quote_24h: f64,
    pub open_24h: f64,
    pub timestamp: DateTime<Utc>,
}

/// Universal kline/candlestick data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalKline {
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub open_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub quote_volume: f64,
    pub trades_count: u64,
    pub taker_buy_volume: f64,
    pub taker_buy_quote_volume: f64,
}

/// Kline interval enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KlineInterval {
    OneSecond,
    OneMinute,
    ThreeMinutes,
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    TwoHours,
    FourHours,
    SixHours,
    EightHours,
    TwelveHours,
    OneDay,
    ThreeDays,
    OneWeek,
    OneMonth,
}

impl fmt::Display for KlineInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::OneSecond => "1s",
            Self::OneMinute => "1m",
            Self::ThreeMinutes => "3m",
            Self::FiveMinutes => "5m",
            Self::FifteenMinutes => "15m",
            Self::ThirtyMinutes => "30m",
            Self::OneHour => "1h",
            Self::TwoHours => "2h",
            Self::FourHours => "4h",
            Self::SixHours => "6h",
            Self::EightHours => "8h",
            Self::TwelveHours => "12h",
            Self::OneDay => "1d",
            Self::ThreeDays => "3d",
            Self::OneWeek => "1w",
            Self::OneMonth => "1M",
        };
        write!(f, "{}", s)
    }
}

/// Main exchange connector trait
#[async_trait]
pub trait ExchangeConnector: Send + Sync {
    type Config: Clone + Send + Sync;
    
    /// Connect to the exchange with the given configuration
    async fn connect(config: Self::Config) -> ExchangeResult<Self> where Self: Sized;
    
    /// Disconnect from the exchange and clean up resources
    async fn disconnect(&self) -> ExchangeResult<()>;
    
    /// Get account information including permissions and trading status
    async fn get_account_info(&self) -> ExchangeResult<AccountInfo>;
    
    /// Get current account balances for all assets
    async fn get_balances(&self) -> ExchangeResult<Vec<Balance>>;
    
    /// Get balance for a specific asset
    async fn get_balance(&self, asset: &str) -> ExchangeResult<Option<Balance>>;
    
    /// Place a new order
    async fn place_order(&self, order: OrderRequest) -> ExchangeResult<UniversalOrder>;
    
    /// Cancel an existing order by ID
    async fn cancel_order(&self, order_id: &str) -> ExchangeResult<()>;
    
    /// Cancel all orders for a symbol (optional symbol filter)
    async fn cancel_all_orders(&self, symbol: Option<&Symbol>) -> ExchangeResult<Vec<String>>;
    
    /// Get order status by ID
    async fn get_order(&self, order_id: &str) -> ExchangeResult<UniversalOrder>;
    
    /// Get all open orders (optional symbol filter)
    async fn get_open_orders(&self, symbol: Option<&Symbol>) -> ExchangeResult<Vec<UniversalOrder>>;
    
    /// Get order history (optional symbol filter)
    async fn get_order_history(&self, symbol: Option<&Symbol>, limit: Option<u32>) -> ExchangeResult<Vec<UniversalOrder>>;
    
    /// Get trade history
    async fn get_trade_history(&self, symbol: Option<&Symbol>, limit: Option<u32>) -> ExchangeResult<Vec<TradeExecution>>;
    
    /// Get current ticker data for a symbol
    async fn get_ticker(&self, symbol: &Symbol) -> ExchangeResult<UniversalTicker>;
    
    /// Get current order book for a symbol
    async fn get_orderbook(&self, symbol: &Symbol, limit: Option<u32>) -> ExchangeResult<UniversalOrderBook>;
    
    /// Get recent trades for a symbol
    async fn get_recent_trades(&self, symbol: &Symbol, limit: Option<u32>) -> ExchangeResult<Vec<UniversalTrade>>;
    
    /// Subscribe to real-time market data streams
    async fn subscribe(&mut self, symbols: Vec<&str>) -> ExchangeResult<()>;
    
    /// Try to receive market data (non-blocking)
    fn try_recv(&mut self) -> Option<UniversalMarketData>;
    
    /// Start the connection processing
    async fn start(&mut self) -> ExchangeResult<()>;
    
    /// Get the exchange name
    fn name(&self) -> &str;
    
    /// Get historical kline/candlestick data
    async fn get_klines(
        &self,
        symbol: &Symbol,
        interval: KlineInterval,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> ExchangeResult<Vec<UniversalKline>>;
    
    /// Test connectivity to the exchange
    async fn ping(&self) -> ExchangeResult<u64>; // Returns ping time in milliseconds
    
    /// Get exchange info (trading rules, symbols, etc.)
    async fn get_exchange_info(&self) -> ExchangeResult<ExchangeInfo>;
}

/// Exchange information and trading rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeInfo {
    pub exchange: Exchange,
    pub timezone: String,
    pub server_time: DateTime<Utc>,
    pub symbols: Vec<SymbolInfo>,
    pub rate_limits: Vec<RateLimit>,
}

/// Symbol trading information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub symbol: Symbol,
    pub base_asset: String,
    pub quote_asset: String,
    pub status: SymbolStatus,
    pub base_precision: u32,
    pub quote_precision: u32,
    pub min_quantity: f64,
    pub max_quantity: f64,
    pub step_size: f64,
    pub min_price: f64,
    pub max_price: f64,
    pub tick_size: f64,
    pub min_notional: f64,
    pub order_types: Vec<OrderType>,
    pub is_spot_trading_allowed: bool,
    pub is_margin_trading_allowed: bool,
}

/// Symbol trading status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SymbolStatus {
    PreTrading,
    Trading,
    PostTrading,
    EndOfDay,
    Halt,
    AuctionMatch,
    Break,
}

/// Rate limit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub rate_type: RateLimitType,
    pub interval: RateLimitInterval,
    pub interval_num: u32,
    pub limit: u32,
}

/// Rate limit types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RateLimitType {
    RequestWeight,
    Orders,
    RawRequests,
}

/// Rate limit intervals
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RateLimitInterval {
    Second,
    Minute,
    Day,
}

/// Conversion trait for exchange-specific to universal data structures
pub trait ToUniversal<T> {
    fn to_universal(&self) -> T;
}

/// Conversion trait for universal to exchange-specific data structures  
pub trait FromUniversal<T> {
    fn from_universal(universal: &T) -> Self;
}

/// Helper trait for validating exchange data
pub trait ExchangeValidation {
    fn validate(&self) -> ExchangeResult<()>;
}

impl ExchangeValidation for OrderRequest {
    fn validate(&self) -> ExchangeResult<()> {
        if !self.symbol.validate() {
            return Err(ExchangeError::InvalidRequest {
                details: format!("Invalid symbol: {}", self.symbol),
            });
        }
        
        if self.quantity <= 0.0 {
            return Err(ExchangeError::InvalidRequest {
                details: "Quantity must be positive".to_string(),
            });
        }
        
        match &self.order_type {
            OrderType::Limit { price } => {
                if *price <= 0.0 {
                    return Err(ExchangeError::InvalidRequest {
                        details: "Limit price must be positive".to_string(),
                    });
                }
            }
            OrderType::StopLimit { stop, limit } => {
                if *stop <= 0.0 || *limit <= 0.0 {
                    return Err(ExchangeError::InvalidRequest {
                        details: "Stop and limit prices must be positive".to_string(),
                    });
                }
            }
            OrderType::Market => {
                // Market orders don't need price validation
            }
        }
        
        Ok(())
    }
}

impl ExchangeValidation for UniversalOrder {
    fn validate(&self) -> ExchangeResult<()> {
        if !self.symbol.validate() {
            return Err(ExchangeError::InvalidRequest {
                details: format!("Invalid symbol: {}", self.symbol),
            });
        }
        
        if self.quantity <= 0.0 {
            return Err(ExchangeError::InvalidRequest {
                details: "Quantity must be positive".to_string(),
            });
        }
        
        if self.filled_quantity < 0.0 || self.filled_quantity > self.quantity {
            return Err(ExchangeError::InvalidRequest {
                details: "Invalid filled quantity".to_string(),
            });
        }
        
        Ok(())
    }
}

/// Helper functions for common exchange operations
pub mod helpers {
    use super::*;
    
    /// Generate a client order ID
    pub fn generate_client_order_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("co_{}", timestamp)
    }
    
    /// Calculate order notional value
    pub fn calculate_notional(quantity: f64, price: f64) -> f64 {
        quantity * price
    }
    
    /// Calculate percentage change
    pub fn calculate_percentage_change(old_value: f64, new_value: f64) -> f64 {
        if old_value == 0.0 {
            0.0
        } else {
            ((new_value - old_value) / old_value) * 100.0
        }
    }
    
    /// Round to specified decimal places
    pub fn round_to_precision(value: f64, precision: u32) -> f64 {
        let multiplier = 10_f64.powi(precision as i32);
        (value * multiplier).round() / multiplier
    }
    
    /// Check if price is within tick size
    pub fn validate_tick_size(price: f64, tick_size: f64) -> bool {
        if tick_size <= 0.0 {
            return true; // No tick size restriction
        }
        (price / tick_size).fract() == 0.0
    }
    
    /// Check if quantity is within step size
    pub fn validate_step_size(quantity: f64, step_size: f64) -> bool {
        if step_size <= 0.0 {
            return true; // No step size restriction
        }
        (quantity / step_size).fract() == 0.0
    }
}