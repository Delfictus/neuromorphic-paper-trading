//! WebSocket streaming interface for real-time market data

use async_trait::async_trait;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{interval, timeout};
use tokio_tungstenite::{
    connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};
use url::Url;

use super::connector::{ExchangeError, ExchangeResult};
use super::types::{Exchange, Symbol, UniversalMarketData, UniversalOrderBook, UniversalQuote, UniversalTrade};

/// WebSocket stream types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StreamType {
    Trade,
    Quote,
    OrderBook,
    Ticker,
    Kline,
    UserData,
}

/// Subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSubscription {
    pub symbol: Symbol,
    pub stream_type: StreamType,
    pub interval: Option<String>, // For kline streams
}

impl StreamSubscription {
    pub fn trade(symbol: Symbol) -> Self {
        Self {
            symbol,
            stream_type: StreamType::Trade,
            interval: None,
        }
    }
    
    pub fn quote(symbol: Symbol) -> Self {
        Self {
            symbol,
            stream_type: StreamType::Quote,
            interval: None,
        }
    }
    
    pub fn orderbook(symbol: Symbol) -> Self {
        Self {
            symbol,
            stream_type: StreamType::OrderBook,
            interval: None,
        }
    }
    
    pub fn kline(symbol: Symbol, interval: String) -> Self {
        Self {
            symbol,
            stream_type: StreamType::Kline,
            interval: Some(interval),
        }
    }
    
    pub fn user_data() -> Self {
        Self {
            symbol: Symbol::new(""), // User data doesn't need symbol
            stream_type: StreamType::UserData,
            interval: None,
        }
    }
}

/// WebSocket connection status
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Stream health metrics
#[derive(Debug, Clone, Default)]
pub struct StreamMetrics {
    pub messages_received: u64,
    pub messages_parsed: u64,
    pub parse_errors: u64,
    pub connection_errors: u64,
    pub reconnection_count: u64,
    pub last_message_time: Option<Instant>,
    pub average_latency_ms: f64,
    pub data_gaps: u64,
}

/// WebSocket configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub base_url: String,
    pub ping_interval: Duration,
    pub reconnect_interval: Duration,
    pub max_reconnect_attempts: u32,
    pub message_timeout: Duration,
    pub buffer_size: usize,
    pub enable_compression: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            base_url: "wss://stream.binance.com:9443/ws".to_string(),
            ping_interval: Duration::from_secs(30),
            reconnect_interval: Duration::from_secs(5),
            max_reconnect_attempts: 10,
            message_timeout: Duration::from_secs(30),
            buffer_size: 1000,
            enable_compression: true,
        }
    }
}

/// Streaming data manager trait
#[async_trait]
pub trait StreamManager: Send + Sync {
    /// Subscribe to a stream
    async fn subscribe(&mut self, subscription: StreamSubscription) -> ExchangeResult<()>;
    
    /// Unsubscribe from a stream
    async fn unsubscribe(&mut self, subscription: StreamSubscription) -> ExchangeResult<()>;
    
    /// Get stream receiver for consuming data
    fn get_receiver(&mut self) -> Option<broadcast::Receiver<UniversalMarketData>>;
    
    /// Get connection status
    async fn get_status(&self) -> ConnectionStatus;
    
    /// Get stream metrics
    async fn get_metrics(&self) -> StreamMetrics;
    
    /// Start the stream manager
    async fn start(&mut self) -> ExchangeResult<()>;
    
    /// Stop the stream manager
    async fn stop(&mut self) -> ExchangeResult<()>;
}

/// WebSocket stream manager implementation
pub struct WebSocketManager {
    config: WebSocketConfig,
    exchange: Exchange,
    subscriptions: Arc<RwLock<HashMap<String, StreamSubscription>>>,
    connection_status: Arc<RwLock<ConnectionStatus>>,
    metrics: Arc<RwLock<StreamMetrics>>,
    data_sender: broadcast::Sender<UniversalMarketData>,
    data_receiver: Option<broadcast::Receiver<UniversalMarketData>>,
    control_sender: Option<mpsc::UnboundedSender<ControlMessage>>,
    websocket_task: Option<tokio::task::JoinHandle<()>>,
}

/// Internal control messages
#[derive(Debug)]
enum ControlMessage {
    Subscribe(StreamSubscription),
    Unsubscribe(StreamSubscription),
    Reconnect,
    Shutdown,
}

impl WebSocketManager {
    pub fn new(config: WebSocketConfig, exchange: Exchange) -> Self {
        let (data_sender, data_receiver) = broadcast::channel(config.buffer_size);
        
        Self {
            config,
            exchange,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            connection_status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            metrics: Arc::new(RwLock::new(StreamMetrics::default())),
            data_sender,
            data_receiver: Some(data_receiver),
            control_sender: None,
            websocket_task: None,
        }
    }
    
    /// Create subscription key for internal tracking
    fn create_subscription_key(subscription: &StreamSubscription) -> String {
        match &subscription.interval {
            Some(interval) => format!("{}:{}:{}", subscription.symbol, subscription.stream_type.as_str(), interval),
            None => format!("{}:{}", subscription.symbol, subscription.stream_type.as_str()),
        }
    }
    
    /// Start the WebSocket connection and message handling
    async fn start_websocket_task(
        config: WebSocketConfig,
        exchange: Exchange,
        subscriptions: Arc<RwLock<HashMap<String, StreamSubscription>>>,
        connection_status: Arc<RwLock<ConnectionStatus>>,
        metrics: Arc<RwLock<StreamMetrics>>,
        data_sender: broadcast::Sender<UniversalMarketData>,
        mut control_receiver: mpsc::UnboundedReceiver<ControlMessage>,
    ) {
        let mut reconnect_attempts = 0;
        let mut websocket: Option<WebSocketStream<MaybeTlsStream<TcpStream>>> = None;
        let mut write_sink: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>> = None;
        
        loop {
            // Update connection status
            *connection_status.write().await = if reconnect_attempts > 0 {
                ConnectionStatus::Reconnecting
            } else {
                ConnectionStatus::Connecting
            };
            
            // Attempt to connect
            match Self::connect_websocket(&config.base_url).await {
                Ok((ws, sink)) => {
                    info!("WebSocket connected successfully");
                    websocket = Some(ws);
                    write_sink = Some(sink);
                    *connection_status.write().await = ConnectionStatus::Connected;
                    reconnect_attempts = 0;
                    
                    // Resubscribe to all active subscriptions
                    let current_subscriptions = subscriptions.read().await.clone();
                    for subscription in current_subscriptions.values() {
                        if let Err(e) = Self::send_subscription_message(&mut write_sink, subscription, true).await {
                            error!("Failed to resubscribe to {}: {}", Self::create_subscription_key(subscription), e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to WebSocket: {}", e);
                    reconnect_attempts += 1;
                    
                    if reconnect_attempts >= config.max_reconnect_attempts {
                        error!("Max reconnection attempts reached, giving up");
                        *connection_status.write().await = ConnectionStatus::Failed;
                        break;
                    }
                    
                    tokio::time::sleep(config.reconnect_interval).await;
                    continue;
                }
            }
            
            // Main message handling loop
            let mut ping_interval = interval(config.ping_interval);
            
            loop {
                tokio::select! {
                    // Handle control messages
                    Some(control_msg) = control_receiver.recv() => {
                        match control_msg {
                            ControlMessage::Subscribe(subscription) => {
                                if let Err(e) = Self::send_subscription_message(&mut write_sink, &subscription, true).await {
                                    error!("Failed to subscribe: {}", e);
                                } else {
                                    let key = Self::create_subscription_key(&subscription);
                                    subscriptions.write().await.insert(key, subscription);
                                }
                            }
                            ControlMessage::Unsubscribe(subscription) => {
                                if let Err(e) = Self::send_subscription_message(&mut write_sink, &subscription, false).await {
                                    error!("Failed to unsubscribe: {}", e);
                                } else {
                                    let key = Self::create_subscription_key(&subscription);
                                    subscriptions.write().await.remove(&key);
                                }
                            }
                            ControlMessage::Reconnect => {
                                info!("Manual reconnection requested");
                                break; // Break inner loop to reconnect
                            }
                            ControlMessage::Shutdown => {
                                info!("Shutdown requested");
                                return;
                            }
                        }
                    }
                    
                    // Handle WebSocket messages
                    msg = Self::receive_message(&mut websocket, &config.message_timeout) => {
                        match msg {
                            Ok(Some(message)) => {
                                metrics.write().await.messages_received += 1;
                                metrics.write().await.last_message_time = Some(Instant::now());
                                
                                if let Err(e) = Self::process_message(
                                    message,
                                    exchange,
                                    &data_sender,
                                    &metrics
                                ).await {
                                    error!("Failed to process message: {}", e);
                                    metrics.write().await.parse_errors += 1;
                                }
                            }
                            Ok(None) => {
                                // Connection closed gracefully
                                warn!("WebSocket connection closed");
                                break;
                            }
                            Err(e) => {
                                error!("WebSocket error: {}", e);
                                metrics.write().await.connection_errors += 1;
                                break;
                            }
                        }
                    }
                    
                    // Send periodic ping
                    _ = ping_interval.tick() => {
                        if let Err(e) = Self::send_ping(&mut write_sink).await {
                            error!("Failed to send ping: {}", e);
                            break;
                        }
                    }
                }
            }
            
            // Connection lost, prepare for reconnection
            websocket = None;
            write_sink = None;
            *connection_status.write().await = ConnectionStatus::Disconnected;
            metrics.write().await.reconnection_count += 1;
            
            warn!("WebSocket disconnected, attempting to reconnect...");
            tokio::time::sleep(config.reconnect_interval).await;
        }
    }
    
    /// Connect to WebSocket
    async fn connect_websocket(
        url: &str,
    ) -> ExchangeResult<(
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    )> {
        let url = Url::parse(url).map_err(|e| ExchangeError::InvalidRequest {
            details: format!("Invalid WebSocket URL: {}", e),
        })?;
        
        let (ws_stream, _) = connect_async(url).await.map_err(|e| ExchangeError::Connection {
            message: format!("WebSocket connection failed: {}", e),
        })?;
        
        let (sink, stream) = ws_stream.split();
        Ok((stream.reunite(sink.reunite(stream).unwrap()).unwrap(), sink))
    }
    
    /// Send subscription/unsubscription message
    async fn send_subscription_message(
        sink: &mut Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
        subscription: &StreamSubscription,
        subscribe: bool,
    ) -> ExchangeResult<()> {
        if let Some(sink) = sink {
            // This is a placeholder - actual implementation would depend on exchange protocol
            let method = if subscribe { "SUBSCRIBE" } else { "UNSUBSCRIBE" };
            let message = serde_json::json!({
                "method": method,
                "params": [format!("{}@{}", subscription.symbol.as_str().to_lowercase(), subscription.stream_type.as_str())],
                "id": chrono::Utc::now().timestamp_millis()
            });
            
            sink.send(Message::Text(message.to_string())).await.map_err(|e| {
                ExchangeError::Connection {
                    message: format!("Failed to send subscription message: {}", e),
                }
            })?;
        }
        
        Ok(())
    }
    
    /// Send ping message
    async fn send_ping(
        sink: &mut Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    ) -> ExchangeResult<()> {
        if let Some(sink) = sink {
            sink.send(Message::Ping(vec![])).await.map_err(|e| ExchangeError::Connection {
                message: format!("Failed to send ping: {}", e),
            })?;
        }
        Ok(())
    }
    
    /// Receive message with timeout
    async fn receive_message(
        websocket: &mut Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        message_timeout: &Duration,
    ) -> ExchangeResult<Option<Message>> {
        if let Some(ws) = websocket {
            match timeout(*message_timeout, ws.next()).await {
                Ok(Some(Ok(message))) => Ok(Some(message)),
                Ok(Some(Err(e))) => Err(ExchangeError::Connection {
                    message: format!("WebSocket error: {}", e),
                }),
                Ok(None) => Ok(None), // Stream ended
                Err(_) => Err(ExchangeError::Timeout {
                    seconds: message_timeout.as_secs(),
                }),
            }
        } else {
            Err(ExchangeError::Connection {
                message: "No active WebSocket connection".to_string(),
            })
        }
    }
    
    /// Process incoming WebSocket message
    async fn process_message(
        message: Message,
        exchange: Exchange,
        data_sender: &broadcast::Sender<UniversalMarketData>,
        metrics: &Arc<RwLock<StreamMetrics>>,
    ) -> ExchangeResult<()> {
        match message {
            Message::Text(text) => {
                // Parse JSON message and convert to UniversalMarketData
                // This is a placeholder - actual implementation would depend on exchange format
                debug!("Received message: {}", text);
                
                // Try to parse as market data
                if let Ok(market_data) = Self::parse_market_data(&text, exchange) {
                    if let Err(_) = data_sender.send(market_data) {
                        // No receivers, that's OK
                    }
                    metrics.write().await.messages_parsed += 1;
                }
            }
            Message::Binary(_) => {
                // Handle binary messages if needed
                debug!("Received binary message");
            }
            Message::Pong(_) => {
                debug!("Received pong");
            }
            Message::Close(frame) => {
                info!("WebSocket close frame: {:?}", frame);
            }
            _ => {
                debug!("Received other message type");
            }
        }
        
        Ok(())
    }
    
    /// Parse market data from JSON text
    fn parse_market_data(text: &str, exchange: Exchange) -> ExchangeResult<UniversalMarketData> {
        // This is a placeholder implementation
        // Real implementation would parse exchange-specific JSON format
        
        let _data: serde_json::Value = serde_json::from_str(text).map_err(|e| {
            ExchangeError::InvalidRequest {
                details: format!("Failed to parse JSON: {}", e),
            }
        })?;
        
        // For now, return a dummy trade
        Ok(UniversalMarketData::Trade(UniversalTrade {
            exchange,
            symbol: Symbol::new("BTC-USD"),
            price: 50000.0,
            quantity: 0.001,
            side: super::types::Side::Buy,
            timestamp_exchange: chrono::Utc::now().timestamp_millis() as u64,
            timestamp_local: chrono::Utc::now().timestamp_millis() as u64,
            trade_id: "test".to_string(),
        }))
    }
}

impl StreamType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trade => "trade",
            Self::Quote => "bookTicker",
            Self::OrderBook => "depth",
            Self::Ticker => "ticker",
            Self::Kline => "kline",
            Self::UserData => "userData",
        }
    }
}

#[async_trait]
impl StreamManager for WebSocketManager {
    async fn subscribe(&mut self, subscription: StreamSubscription) -> ExchangeResult<()> {
        if let Some(sender) = &self.control_sender {
            sender.send(ControlMessage::Subscribe(subscription)).map_err(|e| {
                ExchangeError::Internal {
                    message: format!("Failed to send subscribe command: {}", e),
                }
            })?;
        } else {
            return Err(ExchangeError::Connection {
                message: "Stream manager not started".to_string(),
            });
        }
        Ok(())
    }
    
    async fn unsubscribe(&mut self, subscription: StreamSubscription) -> ExchangeResult<()> {
        if let Some(sender) = &self.control_sender {
            sender.send(ControlMessage::Unsubscribe(subscription)).map_err(|e| {
                ExchangeError::Internal {
                    message: format!("Failed to send unsubscribe command: {}", e),
                }
            })?;
        } else {
            return Err(ExchangeError::Connection {
                message: "Stream manager not started".to_string(),
            });
        }
        Ok(())
    }
    
    fn get_receiver(&mut self) -> Option<broadcast::Receiver<UniversalMarketData>> {
        self.data_receiver.take()
    }
    
    async fn get_status(&self) -> ConnectionStatus {
        *self.connection_status.read().await
    }
    
    async fn get_metrics(&self) -> StreamMetrics {
        self.metrics.read().await.clone()
    }
    
    async fn start(&mut self) -> ExchangeResult<()> {
        if self.websocket_task.is_some() {
            return Err(ExchangeError::InvalidRequest {
                details: "Stream manager already started".to_string(),
            });
        }
        
        let (control_sender, control_receiver) = mpsc::unbounded_channel();
        self.control_sender = Some(control_sender);
        
        let config = self.config.clone();
        let exchange = self.exchange.clone();
        let subscriptions = self.subscriptions.clone();
        let connection_status = self.connection_status.clone();
        let metrics = self.metrics.clone();
        let data_sender = self.data_sender.clone();
        
        let task = tokio::spawn(async move {
            Self::start_websocket_task(
                config,
                exchange,
                subscriptions,
                connection_status,
                metrics,
                data_sender,
                control_receiver,
            ).await;
        });
        
        self.websocket_task = Some(task);
        
        Ok(())
    }
    
    async fn stop(&mut self) -> ExchangeResult<()> {
        if let Some(sender) = &self.control_sender {
            let _ = sender.send(ControlMessage::Shutdown);
        }
        
        if let Some(task) = self.websocket_task.take() {
            task.abort();
        }
        
        self.control_sender = None;
        *self.connection_status.write().await = ConnectionStatus::Disconnected;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_websocket_manager_creation() {
        let config = WebSocketConfig::default();
        let manager = WebSocketManager::new(config, Exchange::Binance);
        
        assert_eq!(manager.get_status().await, ConnectionStatus::Disconnected);
    }
    
    #[tokio::test]
    async fn test_subscription_key_generation() {
        let subscription = StreamSubscription::trade(Symbol::new("BTC-USD"));
        let key = WebSocketManager::create_subscription_key(&subscription);
        assert_eq!(key, "BTC-USD:trade");
        
        let kline_subscription = StreamSubscription::kline(Symbol::new("ETH-USD"), "1m".to_string());
        let kline_key = WebSocketManager::create_subscription_key(&kline_subscription);
        assert_eq!(kline_key, "ETH-USD:kline:1m");
    }
}