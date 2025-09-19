//! Binance exchange connector

use super::{ExchangeConnector, ExchangeError, ExchangeResult, Exchange, Symbol, Side, UniversalMarketData, UniversalTrade, UniversalQuote, UniversalOrderBook};
use async_trait::async_trait;

#[derive(Clone)]
pub struct BinanceConfig {
    pub base_url: String,
}

impl Default for BinanceConfig {
    fn default() -> Self {
        Self {
            base_url: "wss://stream.binance.com:9443/ws".to_string(),
        }
    }
}
use anyhow::Result;
use dashmap::DashMap;
use futures_util::{StreamExt, SinkExt};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;

/// Binance trade message
#[derive(Deserialize, Debug, Clone)]
pub struct BinanceTrade {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "T")]
    pub trade_time: u64,
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
    #[serde(rename = "t")]
    pub trade_id: u64,
}

/// Multi-symbol tracker
pub struct MultiSymbolTracker {
    pub trades_per_symbol: Arc<DashMap<String, AtomicU64>>,
    pub messages_per_second: Arc<AtomicU64>,
    pub total_messages: Arc<AtomicU64>,
    pub start_time: Instant,
}

impl MultiSymbolTracker {
    pub fn new() -> Self {
        Self {
            trades_per_symbol: Arc::new(DashMap::new()),
            messages_per_second: Arc::new(AtomicU64::new(0)),
            total_messages: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        }
    }
    
    pub fn record_trade(&self, symbol: &str) {
        self.trades_per_symbol
            .entry(symbol.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
        self.total_messages.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_stats(&self) -> (u64, f64) {
        let total = self.total_messages.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let rate = if elapsed > 0.0 { total as f64 / elapsed } else { 0.0 };
        (total, rate)
    }
    
    pub fn print_stats(&self) {
        let (total, rate) = self.get_stats();
        println!("Total messages: {}, Rate: {:.2} msg/s", total, rate);
        
        for entry in self.trades_per_symbol.iter() {
            let symbol = entry.key();
            let count = entry.value().load(Ordering::Relaxed);
            println!("  Symbol {}: {} trades", symbol, count);
        }
    }
}

/// Binance WebSocket connector
pub struct BinanceWebSocket {
    url: String,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    last_ping: Instant,
    reconnect_attempts: u32,
    tracker: Arc<MultiSymbolTracker>,
    data_channel: mpsc::UnboundedSender<UniversalMarketData>,
    data_receiver: Arc<RwLock<mpsc::UnboundedReceiver<UniversalMarketData>>>,
    subscribed_symbols: Vec<String>,
}

impl BinanceWebSocket {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            url: "wss://stream.binance.com:9443/ws".to_string(),
            ws_stream: None,
            last_ping: Instant::now(),
            reconnect_attempts: 0,
            tracker: Arc::new(MultiSymbolTracker::new()),
            data_channel: tx,
            data_receiver: Arc::new(RwLock::new(rx)),
            subscribed_symbols: Vec::new(),
        }
    }
    
    pub async fn connect_ws(&mut self) -> Result<(), ExchangeError> {
        println!("Connecting to Binance...");
        
        let url = url::Url::parse(&self.url)
            .map_err(|e| ExchangeError::Connection { message: e.to_string() })?;
        
        let (ws_stream, _) = connect_async(url).await?;
        println!("Connected to Binance WebSocket");
        
        self.ws_stream = Some(ws_stream);
        self.last_ping = Instant::now();
        self.reconnect_attempts = 0;
        
        Ok(())
    }
    
    pub async fn ping(&mut self) -> Result<(), ExchangeError> {
        let msg = json!({
            "ping": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        });
        
        if let Some(ws) = &mut self.ws_stream {
            ws.send(Message::Text(msg.to_string())).await?;
            self.last_ping = Instant::now();
        }
        Ok(())
    }
    
    pub async fn subscribe_trades(&mut self, symbol: &str) -> Result<(), ExchangeError> {
        let subscribe_msg = json!({
            "method": "SUBSCRIBE",
            "params": [format!("{}@trade", symbol.to_lowercase())],
            "id": 1
        });
        
        if let Some(ws) = &mut self.ws_stream {
            ws.send(Message::Text(subscribe_msg.to_string())).await?;
            
            // Wait for confirmation
            if let Some(msg) = ws.next().await {
                let msg = msg?;
                if let Message::Text(text) = msg {
                    println!("Subscription response: {}", text);
                }
            }
            
            self.subscribed_symbols.push(symbol.to_string());
        }
        Ok(())
    }
    
    pub async fn subscribe_multiple(&mut self, symbols: Vec<&str>) -> Result<(), ExchangeError> {
        let params: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@trade", s.to_lowercase()))
            .collect();
        
        let subscribe_msg = json!({
            "method": "SUBSCRIBE",
            "params": params,
            "id": 2
        });
        
        if let Some(ws) = &mut self.ws_stream {
            ws.send(Message::Text(subscribe_msg.to_string())).await?;
            time::sleep(Duration::from_millis(100)).await;
            
            for symbol in symbols {
                self.subscribed_symbols.push(symbol.to_string());
            }
        }
        Ok(())
    }
    
    pub async fn receive_trade(&mut self) -> Result<BinanceTrade, ExchangeError> {
        if let Some(ws) = &mut self.ws_stream {
            while let Some(msg) = ws.next().await {
                let msg = msg?;
                if let Message::Text(text) = msg {
                    if text.contains("\"e\":\"trade\"") {
                        let trade: BinanceTrade = serde_json::from_str(&text)?;
                        self.tracker.record_trade(&trade.symbol);
                        return Ok(trade);
                    } else if text.contains("pong") {
                        println!("Received pong");
                    }
                }
            }
        }
        Err(ExchangeError::Connection { message: "No trade received".to_string() })
    }
    
    async fn process_messages(&mut self) {
        while let Some(ws) = &mut self.ws_stream {
            tokio::select! {
                msg = ws.next() => {
                    if let Some(Ok(Message::Text(text))) = msg {
                        if let Ok(trade) = serde_json::from_str::<BinanceTrade>(&text) {
                            self.tracker.record_trade(&trade.symbol);
                            
                            // Convert to universal format
                            let universal = UniversalTrade {
                                exchange: Exchange::Binance,
                                symbol: Symbol::new(trade.symbol.clone()),
                                price: trade.price.parse().unwrap_or(0.0),
                                quantity: trade.quantity.parse().unwrap_or(0.0),
                                side: if trade.is_buyer_maker { Side::Sell } else { Side::Buy },
                                timestamp_exchange: trade.trade_time,
                                timestamp_local: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as u64,
                                trade_id: trade.trade_id.to_string(),
                            };
                            
                            let _ = self.data_channel.send(UniversalMarketData::Trade(universal));
                        }
                    }
                }
                _ = time::sleep(Duration::from_secs(30)) => {
                    // Send ping
                    let _ = self.ping().await;
                }
            }
        }
    }
    
    pub fn get_tracker(&self) -> Arc<MultiSymbolTracker> {
        self.tracker.clone()
    }
}

#[async_trait]
impl ExchangeConnector for BinanceWebSocket {
    type Config = BinanceConfig;
    
    async fn connect(config: Self::Config) -> ExchangeResult<Self> {
        let mut instance = Self::new();
        instance.url = config.base_url;
        instance.connect_ws().await?;
        Ok(instance)
    }
    
    async fn disconnect(&self) -> ExchangeResult<()> {
        // Note: In a real implementation, this would need proper shutdown coordination
        // For now, just return success as the connection will be dropped
        Ok(())
    }
    
    async fn subscribe(&mut self, symbols: Vec<&str>) -> ExchangeResult<()> {
        self.subscribe_multiple(symbols).await?;
        Ok(())
    }
    
    fn try_recv(&mut self) -> Option<UniversalMarketData> {
        self.data_receiver.write().try_recv().ok()
    }
    
    async fn start(&mut self) -> ExchangeResult<()> {
        self.process_messages().await;
        Ok(())
    }
    
    fn name(&self) -> &str {
        "Binance"
    }

    // Stub implementations for trading functionality - to be implemented later
    async fn get_account_info(&self) -> ExchangeResult<super::AccountInfo> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_balances(&self) -> ExchangeResult<Vec<super::Balance>> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_balance(&self, _asset: &str) -> ExchangeResult<Option<super::Balance>> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn place_order(&self, _order: super::OrderRequest) -> ExchangeResult<super::UniversalOrder> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn cancel_order(&self, _order_id: &str) -> ExchangeResult<()> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn cancel_all_orders(&self, _symbol: Option<&Symbol>) -> ExchangeResult<Vec<String>> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_order(&self, _order_id: &str) -> ExchangeResult<super::UniversalOrder> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_open_orders(&self, _symbol: Option<&Symbol>) -> ExchangeResult<Vec<super::UniversalOrder>> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_order_history(&self, _symbol: Option<&Symbol>, _limit: Option<u32>) -> ExchangeResult<Vec<super::UniversalOrder>> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_trade_history(&self, _symbol: Option<&Symbol>, _limit: Option<u32>) -> ExchangeResult<Vec<super::TradeExecution>> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_ticker(&self, _symbol: &Symbol) -> ExchangeResult<super::UniversalTicker> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_orderbook(&self, _symbol: &Symbol, _limit: Option<u32>) -> ExchangeResult<UniversalOrderBook> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_recent_trades(&self, _symbol: &Symbol, _limit: Option<u32>) -> ExchangeResult<Vec<UniversalTrade>> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_klines(&self, _symbol: &Symbol, _interval: super::KlineInterval, _start_time: Option<chrono::DateTime<chrono::Utc>>, _end_time: Option<chrono::DateTime<chrono::Utc>>, _limit: Option<u32>) -> ExchangeResult<Vec<super::UniversalKline>> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn ping(&self) -> ExchangeResult<u64> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }

    async fn get_exchange_info(&self) -> ExchangeResult<super::ExchangeInfo> {
        Err(ExchangeError::Internal { message: "Not implemented".to_string() })
    }
}