//! Unified market data feed combining all exchanges

use super::{MarketDataNormalizer, SymbolMapper, TimeSynchronizer};
use super::normalizers::{BinanceNormalizer, CoinbaseNormalizer};
use crate::exchanges::{
    Exchange, Symbol, UniversalTrade, UniversalQuote, 
    UniversalOrderBook, MarketDataType
};
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::{Duration, Instant};

/// Unified market data event
#[derive(Clone, Debug)]
pub enum UnifiedMarketEvent {
    Trade(UniversalTrade),
    Quote(UniversalQuote),
    OrderBook(UniversalOrderBook),
    Heartbeat { exchange: Exchange, timestamp: u64 },
    Error { exchange: Exchange, error: String },
}

/// Feed statistics
#[derive(Default, Clone, Debug)]
pub struct FeedStatistics {
    pub messages_received: u64,
    pub messages_processed: u64,
    pub errors_count: u64,
    pub last_update: Option<Instant>,
    pub latency_ms: f64,
}

/// Unified feed configuration
pub struct UnifiedFeedConfig {
    pub buffer_size: usize,
    pub heartbeat_interval: Duration,
    pub max_latency_ms: f64,
    pub enable_deduplication: bool,
}

impl Default for UnifiedFeedConfig {
    fn default() -> Self {
        Self {
            buffer_size: 10000,
            heartbeat_interval: Duration::from_secs(30),
            max_latency_ms: 100.0,
            enable_deduplication: true,
        }
    }
}

/// Unified market data feed
pub struct UnifiedMarketFeed {
    normalizers: DashMap<Exchange, Arc<dyn MarketDataNormalizer>>,
    symbol_mapper: Arc<SymbolMapper>,
    time_sync: Arc<TimeSynchronizer>,
    event_sender: mpsc::UnboundedSender<UnifiedMarketEvent>,
    event_receiver: Option<mpsc::UnboundedReceiver<UnifiedMarketEvent>>,
    statistics: DashMap<Exchange, FeedStatistics>,
    config: UnifiedFeedConfig,
    dedup_cache: DashMap<String, Instant>,
}

impl UnifiedMarketFeed {
    pub fn new(config: UnifiedFeedConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let symbol_mapper = Arc::new(SymbolMapper::new());
        let time_sync = Arc::new(TimeSynchronizer::new());
        
        let mut feed = Self {
            normalizers: DashMap::new(),
            symbol_mapper: symbol_mapper.clone(),
            time_sync: time_sync.clone(),
            event_sender: tx,
            event_receiver: Some(rx),
            statistics: DashMap::new(),
            config,
            dedup_cache: DashMap::new(),
        };
        
        // Register default normalizers
        feed.register_normalizer(
            Exchange::Binance,
            Arc::new(BinanceNormalizer::new(
                symbol_mapper.clone(),
                time_sync.clone()
            ))
        );
        
        feed.register_normalizer(
            Exchange::Coinbase,
            Arc::new(CoinbaseNormalizer::new(
                symbol_mapper.clone(),
                time_sync.clone()
            ))
        );
        
        feed
    }
    
    /// Register a normalizer for an exchange
    pub fn register_normalizer(
        &mut self,
        exchange: Exchange,
        normalizer: Arc<dyn MarketDataNormalizer>
    ) {
        self.normalizers.insert(exchange, normalizer);
        self.statistics.insert(exchange, FeedStatistics::default());
    }
    
    /// Process raw market data from an exchange
    pub async fn process_raw_data(
        &self,
        exchange: Exchange,
        data_type: MarketDataType,
        raw_data: &[u8]
    ) -> Result<()> {
        let normalizer = self.normalizers
            .get(&exchange)
            .ok_or_else(|| anyhow::anyhow!("No normalizer for {:?}", exchange))?;
        
        let start = Instant::now();
        
        // Normalize based on data type
        let event = match data_type {
            MarketDataType::Trade => {
                let trade = normalizer.normalize_trade(raw_data)?;
                
                // Deduplicate if enabled
                if self.config.enable_deduplication {
                    let key = format!("{}:{}:{}", exchange, trade.symbol, trade.trade_id);
                    if let Some(last_seen) = self.dedup_cache.get(&key) {
                        if last_seen.elapsed() < Duration::from_secs(1) {
                            return Ok(()); // Skip duplicate
                        }
                    }
                    self.dedup_cache.insert(key, Instant::now());
                }
                
                UnifiedMarketEvent::Trade(trade)
            }
            MarketDataType::Quote => {
                let quote = normalizer.normalize_quote(raw_data)?;
                UnifiedMarketEvent::Quote(quote)
            }
            MarketDataType::OrderBook => {
                let book = normalizer.normalize_book(raw_data)?;
                UnifiedMarketEvent::OrderBook(book)
            }
        };
        
        // Send normalized event
        self.event_sender.send(event)?;
        
        // Update statistics
        if let Some(mut stats) = self.statistics.get_mut(&exchange) {
            stats.messages_received += 1;
            stats.messages_processed += 1;
            stats.last_update = Some(Instant::now());
            stats.latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        }
        
        // Clean old dedup entries periodically
        if self.dedup_cache.len() > 10000 {
            self.clean_dedup_cache();
        }
        
        Ok(())
    }
    
    /// Clean old entries from deduplication cache
    fn clean_dedup_cache(&self) {
        let cutoff = Instant::now() - Duration::from_secs(60);
        self.dedup_cache.retain(|_, instant| *instant > cutoff);
    }
    
    /// Subscribe to unified market events
    pub fn subscribe(&mut self) -> Option<mpsc::UnboundedReceiver<UnifiedMarketEvent>> {
        self.event_receiver.take()
    }
    
    /// Get feed statistics
    pub fn get_statistics(&self, exchange: Exchange) -> Option<FeedStatistics> {
        self.statistics.get(&exchange).map(|s| s.clone())
    }
    
    /// Get all statistics
    pub fn get_all_statistics(&self) -> Vec<(Exchange, FeedStatistics)> {
        self.statistics
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }
    
    /// Check feed health
    pub fn is_healthy(&self, exchange: Exchange) -> bool {
        self.statistics
            .get(&exchange)
            .map(|stats| {
                if let Some(last_update) = stats.last_update {
                    last_update.elapsed() < self.config.heartbeat_interval * 2
                        && stats.latency_ms < self.config.max_latency_ms
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }
    
    /// Send heartbeat for an exchange
    pub async fn send_heartbeat(&self, exchange: Exchange) -> Result<()> {
        let timestamp = self.time_sync.get_local_time();
        let event = UnifiedMarketEvent::Heartbeat { exchange, timestamp };
        self.event_sender.send(event)?;
        Ok(())
    }
    
    /// Handle error from exchange
    pub async fn handle_error(&self, exchange: Exchange, error: String) -> Result<()> {
        let event = UnifiedMarketEvent::Error { exchange, error: error.clone() };
        self.event_sender.send(event)?;
        
        // Update error count
        if let Some(mut stats) = self.statistics.get_mut(&exchange) {
            stats.errors_count += 1;
        }
        
        Ok(())
    }
}

/// Aggregated market data across exchanges
pub struct AggregatedMarketData {
    trades_by_symbol: DashMap<Symbol, Vec<UniversalTrade>>,
    quotes_by_symbol: DashMap<Symbol, Vec<UniversalQuote>>,
    books_by_symbol: DashMap<Symbol, Vec<UniversalOrderBook>>,
    aggregation_window: Duration,
}

impl AggregatedMarketData {
    pub fn new(window: Duration) -> Self {
        Self {
            trades_by_symbol: DashMap::new(),
            quotes_by_symbol: DashMap::new(),
            books_by_symbol: DashMap::new(),
            aggregation_window: window,
        }
    }
    
    /// Add trade to aggregation
    pub fn add_trade(&self, trade: UniversalTrade) {
        self.trades_by_symbol
            .entry(trade.symbol.clone())
            .or_insert_with(Vec::new)
            .push(trade);
    }
    
    /// Add quote to aggregation
    pub fn add_quote(&self, quote: UniversalQuote) {
        self.quotes_by_symbol
            .entry(quote.symbol.clone())
            .or_insert_with(Vec::new)
            .push(quote);
    }
    
    /// Add order book to aggregation
    pub fn add_orderbook(&self, book: UniversalOrderBook) {
        self.books_by_symbol
            .entry(book.symbol.clone())
            .or_insert_with(Vec::new)
            .push(book);
    }
    
    /// Get best bid across all exchanges for a symbol
    pub fn get_best_bid(&self, symbol: &Symbol) -> Option<(f64, f64, Exchange)> {
        self.quotes_by_symbol
            .get(symbol)
            .and_then(|quotes| {
                quotes.iter()
                    .filter(|q| q.bid_price > 0.0)
                    .max_by(|a, b| a.bid_price.partial_cmp(&b.bid_price).unwrap())
                    .map(|q| (q.bid_price, q.bid_size, q.exchange))
            })
    }
    
    /// Get best ask across all exchanges for a symbol
    pub fn get_best_ask(&self, symbol: &Symbol) -> Option<(f64, f64, Exchange)> {
        self.quotes_by_symbol
            .get(symbol)
            .and_then(|quotes| {
                quotes.iter()
                    .filter(|q| q.ask_price > 0.0)
                    .min_by(|a, b| a.ask_price.partial_cmp(&b.ask_price).unwrap())
                    .map(|q| (q.ask_price, q.ask_size, q.exchange))
            })
    }
    
    /// Calculate VWAP across exchanges
    pub fn calculate_vwap(&self, symbol: &Symbol) -> Option<f64> {
        self.trades_by_symbol
            .get(symbol)
            .and_then(|trades| {
                if trades.is_empty() {
                    return None;
                }
                
                let total_value: f64 = trades.iter()
                    .map(|t| t.price * t.quantity)
                    .sum();
                
                let total_volume: f64 = trades.iter()
                    .map(|t| t.quantity)
                    .sum();
                
                if total_volume > 0.0 {
                    Some(total_value / total_volume)
                } else {
                    None
                }
            })
    }
    
    /// Clear old data outside aggregation window
    pub fn clean_old_data(&self) {
        let cutoff = self.time_sync.get_local_time() - 
            (self.aggregation_window.as_millis() as u64);
        
        // Clean trades
        for mut entry in self.trades_by_symbol.iter_mut() {
            entry.value_mut().retain(|t| t.timestamp_local > cutoff);
        }
        
        // Clean quotes
        for mut entry in self.quotes_by_symbol.iter_mut() {
            entry.value_mut().retain(|q| q.timestamp_local > cutoff);
        }
        
        // Clean books
        for mut entry in self.books_by_symbol.iter_mut() {
            entry.value_mut().retain(|b| b.timestamp_local > cutoff);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unified_feed_creation() {
        let config = UnifiedFeedConfig::default();
        let mut feed = UnifiedMarketFeed::new(config);
        assert!(feed.subscribe().is_some());
    }
    
    #[test]
    fn test_aggregated_data() {
        let aggregated = AggregatedMarketData::new(Duration::from_secs(60));
        
        let trade = UniversalTrade {
            exchange: Exchange::Binance,
            symbol: Symbol::new("BTC-USD"),
            price: 50000.0,
            quantity: 1.0,
            side: crate::exchanges::Side::Buy,
            timestamp_exchange: 0,
            timestamp_local: 0,
            trade_id: "test".to_string(),
        };
        
        aggregated.add_trade(trade);
        
        let vwap = aggregated.calculate_vwap(&Symbol::new("BTC-USD"));
        assert_eq!(vwap, Some(50000.0));
    }
}