//! Binance WebSocket implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use super::connector::{ExchangeError, ExchangeResult};
use super::types::{Exchange, Side, Symbol, UniversalMarketData, UniversalOrderBook, UniversalQuote, UniversalTrade};
use super::websocket::{
    ConnectionStatus, StreamManager, StreamMetrics, StreamSubscription, StreamType, WebSocketConfig, WebSocketManager,
};

/// Binance WebSocket stream manager
pub struct BinanceWebSocketManager {
    inner: WebSocketManager,
    testnet: bool,
}

/// Binance WebSocket message format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinanceMessage {
    stream: String,
    data: serde_json::Value,
}

/// Binance trade data format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinanceTradeData {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "t")]
    trade_id: i64,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    quantity: String,
    #[serde(rename = "b")]
    buyer_order_id: i64,
    #[serde(rename = "a")]
    seller_order_id: i64,
    #[serde(rename = "T")]
    trade_time: i64,
    #[serde(rename = "m")]
    is_buyer_maker: bool,
}

/// Binance book ticker data format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinanceBookTickerData {
    #[serde(rename = "u")]
    order_book_update_id: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "b")]
    best_bid_price: String,
    #[serde(rename = "B")]
    best_bid_qty: String,
    #[serde(rename = "a")]
    best_ask_price: String,
    #[serde(rename = "A")]
    best_ask_qty: String,
}

/// Binance order book data format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinanceDepthData {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "U")]
    first_update_id: i64,
    #[serde(rename = "u")]
    final_update_id: i64,
    #[serde(rename = "b")]
    bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    asks: Vec<[String; 2]>,
}

/// Binance 24hr ticker data format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinanceTickerData {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "p")]
    price_change: String,
    #[serde(rename = "P")]
    price_change_percent: String,
    #[serde(rename = "w")]
    weighted_avg_price: String,
    #[serde(rename = "x")]
    prev_close_price: String,
    #[serde(rename = "c")]
    last_price: String,
    #[serde(rename = "Q")]
    last_qty: String,
    #[serde(rename = "b")]
    best_bid_price: String,
    #[serde(rename = "B")]
    best_bid_qty: String,
    #[serde(rename = "a")]
    best_ask_price: String,
    #[serde(rename = "A")]
    best_ask_qty: String,
    #[serde(rename = "o")]
    open_price: String,
    #[serde(rename = "h")]
    high_price: String,
    #[serde(rename = "l")]
    low_price: String,
    #[serde(rename = "v")]
    total_traded_base_asset_volume: String,
    #[serde(rename = "q")]
    total_traded_quote_asset_volume: String,
    #[serde(rename = "O")]
    statistics_open_time: i64,
    #[serde(rename = "C")]
    statistics_close_time: i64,
    #[serde(rename = "F")]
    first_trade_id: i64,
    #[serde(rename = "L")]
    last_trade_id: i64,
    #[serde(rename = "n")]
    total_trades: i64,
}

impl BinanceWebSocketManager {
    /// Create a new Binance WebSocket manager
    pub fn new(testnet: bool) -> Self {
        let base_url = if testnet {
            "wss://testnet.binance.vision/ws".to_string()
        } else {
            "wss://stream.binance.com:9443/ws".to_string()
        };
        
        let config = WebSocketConfig {
            base_url,
            ping_interval: Duration::from_secs(180), // Binance expects 3 minute intervals
            reconnect_interval: Duration::from_secs(5),
            max_reconnect_attempts: 10,
            message_timeout: Duration::from_secs(30),
            buffer_size: 1000,
            enable_compression: true,
        };
        
        Self {
            inner: WebSocketManager::new(config, Exchange::Binance),
            testnet,
        }
    }
    
    /// Create subscription for Binance format
    fn create_binance_subscription(&self, subscription: &StreamSubscription) -> String {
        let symbol = subscription.symbol.as_str().to_lowercase();
        
        match subscription.stream_type {
            StreamType::Trade => format!("{}@trade", symbol),
            StreamType::Quote => format!("{}@bookTicker", symbol),
            StreamType::OrderBook => format!("{}@depth", symbol),
            StreamType::Ticker => format!("{}@ticker", symbol),
            StreamType::Kline => {
                let default_interval = "1m".to_string();
                let interval = subscription.interval.as_ref().unwrap_or(&default_interval);
                format!("{}@kline_{}", symbol, interval)
            }
            StreamType::UserData => {
                // User data streams require a listen key from the REST API
                "userData".to_string()
            }
        }
    }
    
    /// Parse Binance WebSocket message
    fn parse_binance_message(&self, text: &str) -> ExchangeResult<Option<UniversalMarketData>> {
        debug!("Parsing Binance message: {}", text);
        
        // Try to parse as Binance message format
        if let Ok(msg) = serde_json::from_str::<BinanceMessage>(text) {
            return self.parse_binance_stream_data(&msg.stream, &msg.data);
        }
        
        // Try to parse as direct stream data (for single stream connections)
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(event_type) = data.get("e").and_then(|v| v.as_str()) {
                return self.parse_binance_event_data(event_type, &data);
            }
        }
        
        debug!("Could not parse message as known Binance format");
        Ok(None)
    }
    
    /// Parse Binance stream data
    fn parse_binance_stream_data(&self, stream: &str, data: &serde_json::Value) -> ExchangeResult<Option<UniversalMarketData>> {
        if stream.contains("@trade") {
            self.parse_trade_data(data)
        } else if stream.contains("@bookTicker") {
            self.parse_quote_data(data)
        } else if stream.contains("@depth") {
            self.parse_depth_data(data)
        } else if stream.contains("@ticker") {
            self.parse_ticker_data(data)
        } else if stream.contains("@kline") {
            self.parse_kline_data(data)
        } else {
            debug!("Unknown stream type: {}", stream);
            Ok(None)
        }
    }
    
    /// Parse Binance event data
    fn parse_binance_event_data(&self, event_type: &str, data: &serde_json::Value) -> ExchangeResult<Option<UniversalMarketData>> {
        match event_type {
            "trade" => self.parse_trade_data(data),
            "24hrTicker" => self.parse_ticker_data(data),
            "depthUpdate" => self.parse_depth_data(data),
            "kline" => self.parse_kline_data(data),
            _ => {
                debug!("Unknown event type: {}", event_type);
                Ok(None)
            }
        }
    }
    
    /// Parse trade data
    fn parse_trade_data(&self, data: &serde_json::Value) -> ExchangeResult<Option<UniversalMarketData>> {
        let trade_data: BinanceTradeData = serde_json::from_value(data.clone()).map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Failed to parse trade data: {}", e),
            }
        })?;
        
        let price = trade_data.price.parse::<f64>().map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Invalid price: {}", e),
            }
        })?;
        
        let quantity = trade_data.quantity.parse::<f64>().map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Invalid quantity: {}", e),
            }
        })?;
        
        let side = if trade_data.is_buyer_maker {
            Side::Sell // Maker is selling, taker is buying
        } else {
            Side::Buy // Maker is buying, taker is selling
        };
        
        let trade = UniversalTrade {
            exchange: Exchange::Binance,
            symbol: Symbol::new(trade_data.symbol),
            price,
            quantity,
            side,
            timestamp_exchange: trade_data.trade_time as u64,
            timestamp_local: chrono::Utc::now().timestamp_millis() as u64,
            trade_id: trade_data.trade_id.to_string(),
        };
        
        Ok(Some(UniversalMarketData::Trade(trade)))
    }
    
    /// Parse quote data
    fn parse_quote_data(&self, data: &serde_json::Value) -> ExchangeResult<Option<UniversalMarketData>> {
        let quote_data: BinanceBookTickerData = serde_json::from_value(data.clone()).map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Failed to parse quote data: {}", e),
            }
        })?;
        
        let bid_price = quote_data.best_bid_price.parse::<f64>().map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Invalid bid price: {}", e),
            }
        })?;
        
        let bid_size = quote_data.best_bid_qty.parse::<f64>().map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Invalid bid size: {}", e),
            }
        })?;
        
        let ask_price = quote_data.best_ask_price.parse::<f64>().map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Invalid ask price: {}", e),
            }
        })?;
        
        let ask_size = quote_data.best_ask_qty.parse::<f64>().map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Invalid ask size: {}", e),
            }
        })?;
        
        let quote = UniversalQuote {
            exchange: Exchange::Binance,
            symbol: Symbol::new(quote_data.symbol),
            bid_price,
            bid_size,
            ask_price,
            ask_size,
            timestamp_exchange: chrono::Utc::now().timestamp_millis() as u64,
            timestamp_local: chrono::Utc::now().timestamp_millis() as u64,
        };
        
        Ok(Some(UniversalMarketData::Quote(quote)))
    }
    
    /// Parse depth data
    fn parse_depth_data(&self, data: &serde_json::Value) -> ExchangeResult<Option<UniversalMarketData>> {
        let depth_data: BinanceDepthData = serde_json::from_value(data.clone()).map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Failed to parse depth data: {}", e),
            }
        })?;
        
        let mut bids = Vec::new();
        for bid in &depth_data.bids {
            let price = bid[0].parse::<f64>().map_err(|e| {
                ExchangeError::InvalidRequest {
                    details: format!("Invalid bid price: {}", e),
                }
            })?;
            let size = bid[1].parse::<f64>().map_err(|e| {
                ExchangeError::InvalidRequest {
                    details: format!("Invalid bid size: {}", e),
                }
            })?;
            bids.push((price, size));
        }
        
        let mut asks = Vec::new();
        for ask in &depth_data.asks {
            let price = ask[0].parse::<f64>().map_err(|e| {
                ExchangeError::InvalidRequest {
                    details: format!("Invalid ask price: {}", e),
                }
            })?;
            let size = ask[1].parse::<f64>().map_err(|e| {
                ExchangeError::InvalidRequest {
                    details: format!("Invalid ask size: {}", e),
                }
            })?;
            asks.push((price, size));
        }
        
        let orderbook = UniversalOrderBook {
            exchange: Exchange::Binance,
            symbol: Symbol::new(depth_data.symbol),
            bids,
            asks,
            timestamp_exchange: depth_data.event_time as u64,
            timestamp_local: chrono::Utc::now().timestamp_millis() as u64,
            sequence: depth_data.final_update_id as u64,
        };
        
        Ok(Some(UniversalMarketData::OrderBook(orderbook)))
    }
    
    /// Parse ticker data
    fn parse_ticker_data(&self, data: &serde_json::Value) -> ExchangeResult<Option<UniversalMarketData>> {
        let _ticker_data: BinanceTickerData = serde_json::from_value(data.clone()).map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Failed to parse ticker data: {}", e),
            }
        })?;
        
        // For now, we don't have UniversalTicker in UniversalMarketData
        // This would need to be added to the enum
        debug!("Ticker data parsed but not yet supported in UniversalMarketData");
        Ok(None)
    }
    
    /// Parse kline data
    fn parse_kline_data(&self, _data: &serde_json::Value) -> ExchangeResult<Option<UniversalMarketData>> {
        // Kline parsing would be implemented here
        debug!("Kline data parsing not yet implemented");
        Ok(None)
    }
    
    /// Subscribe to multiple symbols at once
    pub async fn subscribe_symbols(&mut self, symbols: Vec<Symbol>, stream_type: StreamType) -> ExchangeResult<()> {
        for symbol in symbols {
            let subscription = match stream_type {
                StreamType::Trade => StreamSubscription::trade(symbol),
                StreamType::Quote => StreamSubscription::quote(symbol),
                StreamType::OrderBook => StreamSubscription::orderbook(symbol),
                StreamType::Ticker => StreamSubscription {
                    symbol,
                    stream_type: StreamType::Ticker,
                    interval: None,
                },
                StreamType::Kline => StreamSubscription::kline(symbol, "1m".to_string()),
                StreamType::UserData => StreamSubscription::user_data(),
            };
            
            self.subscribe(subscription).await?;
        }
        Ok(())
    }
    
    /// Get Binance-specific stream URL for combined streams
    pub fn get_combined_stream_url(&self, subscriptions: &[String]) -> String {
        let base_url = if self.testnet {
            "wss://testnet.binance.vision/stream"
        } else {
            "wss://stream.binance.com:9443/stream"
        };
        
        let streams = subscriptions.join("/");
        format!("{}?streams={}", base_url, streams)
    }
}

#[async_trait]
impl StreamManager for BinanceWebSocketManager {
    async fn subscribe(&mut self, subscription: StreamSubscription) -> ExchangeResult<()> {
        self.inner.subscribe(subscription).await
    }
    
    async fn unsubscribe(&mut self, subscription: StreamSubscription) -> ExchangeResult<()> {
        self.inner.unsubscribe(subscription).await
    }
    
    fn get_receiver(&mut self) -> Option<tokio::sync::broadcast::Receiver<UniversalMarketData>> {
        self.inner.get_receiver()
    }
    
    async fn get_status(&self) -> ConnectionStatus {
        self.inner.get_status().await
    }
    
    async fn get_metrics(&self) -> StreamMetrics {
        self.inner.get_metrics().await
    }
    
    async fn start(&mut self) -> ExchangeResult<()> {
        info!("Starting Binance WebSocket manager (testnet: {})", self.testnet);
        self.inner.start().await
    }
    
    async fn stop(&mut self) -> ExchangeResult<()> {
        info!("Stopping Binance WebSocket manager");
        self.inner.stop().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_binance_websocket_manager_creation() {
        let manager = BinanceWebSocketManager::new(true);
        assert_eq!(manager.get_status().await, ConnectionStatus::Disconnected);
        assert!(manager.testnet);
    }
    
    #[test]
    fn test_binance_subscription_format() {
        let manager = BinanceWebSocketManager::new(true);
        
        let trade_sub = StreamSubscription::trade(Symbol::new("BTCUSDT"));
        let stream = manager.create_binance_subscription(&trade_sub);
        assert_eq!(stream, "btcusdt@trade");
        
        let quote_sub = StreamSubscription::quote(Symbol::new("ETHUSDT"));
        let stream = manager.create_binance_subscription(&quote_sub);
        assert_eq!(stream, "ethusdt@bookTicker");
        
        let kline_sub = StreamSubscription::kline(Symbol::new("ADAUSDT"), "5m".to_string());
        let stream = manager.create_binance_subscription(&kline_sub);
        assert_eq!(stream, "adausdt@kline_5m");
    }
    
    #[test]
    fn test_parse_binance_trade_data() {
        let manager = BinanceWebSocketManager::new(true);
        
        let trade_json = r#"
        {
            "e": "trade",
            "E": 1672515782136,
            "s": "BTCUSDT",
            "t": 12345,
            "p": "16569.01",
            "q": "0.014",
            "b": 88,
            "a": 50,
            "T": 1672515782134,
            "m": true
        }
        "#;
        
        let data: serde_json::Value = serde_json::from_str(trade_json).unwrap();
        let result = manager.parse_trade_data(&data).unwrap();
        
        assert!(result.is_some());
        if let Some(UniversalMarketData::Trade(trade)) = result {
            assert_eq!(trade.symbol.as_str(), "BTCUSDT");
            assert_eq!(trade.price, 16569.01);
            assert_eq!(trade.quantity, 0.014);
            assert_eq!(trade.side, Side::Sell); // is_buyer_maker = true means sell
        }
    }
}