//! Bridge between market data and spike encoding

use super::unified_feed::{UnifiedMarketEvent, UnifiedMarketFeed};
use ares_spike_encoding::{SpikeEncoder, Spike, SpikePattern, MarketData};
use crate::exchanges::{Symbol, Exchange};
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::{Duration, Instant};

/// Quote data structure for spike encoding
#[derive(Debug, Clone)]
pub struct QuoteData {
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_size: f64,
    pub ask_size: f64,
    pub timestamp: u64,
}

/// Order book data structure for spike encoding
#[derive(Debug, Clone)]
pub struct OrderBookData {
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
    pub timestamp: u64,
}

/// Market state tracking
#[derive(Debug, Default, Clone)]
pub struct MarketState {
    pub last_price: f64,
    pub volume: f64,
    pub volatility: f64,
    pub trend: f64,
}

/// Market state tracker
#[derive(Debug, Default)]
pub struct MarketStateTracker {
    pub state: MarketState,
    price_history: Vec<f64>,
    max_history: usize,
}

impl MarketStateTracker {
    pub fn new() -> Self {
        Self {
            state: MarketState::default(),
            price_history: Vec::new(),
            max_history: 100,
        }
    }

    pub fn update_from_trade(&mut self, price: f64, volume: f64) {
        self.state.last_price = price;
        self.state.volume += volume;
        
        self.price_history.push(price);
        if self.price_history.len() > self.max_history {
            self.price_history.remove(0);
        }
        
        self.update_volatility();
    }

    pub fn update_from_quote(&mut self, bid: f64, ask: f64, volume: f64) {
        let mid_price = (bid + ask) / 2.0;
        self.state.last_price = mid_price;
        self.state.volume += volume;
        
        self.price_history.push(mid_price);
        if self.price_history.len() > self.max_history {
            self.price_history.remove(0);
        }
        
        self.update_volatility();
    }

    fn update_volatility(&mut self) {
        if self.price_history.len() < 2 {
            return;
        }
        
        let returns: Vec<f64> = self.price_history.windows(2)
            .map(|w| (w[1] / w[0] - 1.0).abs())
            .collect();
        
        if !returns.is_empty() {
            self.state.volatility = returns.iter().sum::<f64>() / returns.len() as f64;
        }
    }
}

/// Spike generation statistics
#[derive(Default, Clone, Debug)]
pub struct SpikeStatistics {
    pub events_processed: u64,
    pub spikes_generated: u64,
    pub encoding_latency_us: f64,
    pub last_spike_time: Option<Instant>,
    pub spike_rate_hz: f64,
}

/// Configuration for spike bridge
pub struct SpikeBridgeConfig {
    pub neuron_count: usize,
    pub spike_buffer_size: usize,
    pub batch_size: usize,
    pub encoding_timeout: Duration,
    pub enable_adaptive_encoding: bool,
}

impl Default for SpikeBridgeConfig {
    fn default() -> Self {
        Self {
            neuron_count: 10000,
            spike_buffer_size: 100000,
            batch_size: 100,
            encoding_timeout: Duration::from_millis(10),
            enable_adaptive_encoding: true,
        }
    }
}

/// Bridge between market data and spike encoding
pub struct MarketDataSpikeBridge {
    encoders: DashMap<Symbol, Arc<SpikeEncoder>>,
    state_trackers: DashMap<Symbol, Arc<MarketStateTracker>>,
    spike_sender: mpsc::UnboundedSender<Vec<Spike>>,
    spike_receiver: Option<mpsc::UnboundedReceiver<Vec<Spike>>>,
    statistics: DashMap<Symbol, SpikeStatistics>,
    config: SpikeBridgeConfig,
    spike_buffer: DashMap<Symbol, Vec<Spike>>,
}

impl MarketDataSpikeBridge {
    pub fn new(config: SpikeBridgeConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            encoders: DashMap::new(),
            state_trackers: DashMap::new(),
            spike_sender: tx,
            spike_receiver: Some(rx),
            statistics: DashMap::new(),
            config,
            spike_buffer: DashMap::new(),
        }
    }
    
    /// Initialize encoder for a symbol
    pub fn initialize_symbol(&self, symbol: Symbol) {
        if !self.encoders.contains_key(&symbol) {
            let encoder = SpikeEncoder::new(self.config.neuron_count, 1000.0)
                .expect("Failed to create spike encoder");
            
            self.encoders.insert(symbol.clone(), Arc::new(encoder));
            self.state_trackers.insert(
                symbol.clone(), 
                Arc::new(MarketStateTracker::new())
            );
            self.statistics.insert(symbol.clone(), SpikeStatistics::default());
            self.spike_buffer.insert(symbol, Vec::new());
        }
    }
    
    /// Process unified market event
    pub async fn process_event(&self, event: UnifiedMarketEvent) -> Result<()> {
        let start = Instant::now();
        
        match event {
            UnifiedMarketEvent::Trade(trade) => {
                self.process_trade(trade).await?;
            }
            UnifiedMarketEvent::Quote(quote) => {
                self.process_quote(quote).await?;
            }
            UnifiedMarketEvent::OrderBook(book) => {
                self.process_orderbook(book).await?;
            }
            UnifiedMarketEvent::Heartbeat { .. } => {
                // Heartbeats don't generate spikes
            }
            UnifiedMarketEvent::Error { exchange, error } => {
                println!("Exchange {} error: {}", exchange, error);
            }
        }
        
        Ok(())
    }
    
    /// Process trade and generate spikes
    async fn process_trade(&self, trade: crate::exchanges::UniversalTrade) -> Result<()> {
        self.initialize_symbol(trade.symbol.clone());
        
        // Update market state
        if let Some(tracker) = self.state_trackers.get(&trade.symbol) {
            let mut tracker = Arc::clone(&tracker.value());
            let tracker_mut = Arc::get_mut(&mut tracker).unwrap();
            tracker_mut.update_from_trade(trade.price, trade.quantity);
        }
        
        // Create market data for encoding
        let market_data = MarketData::new(
            trade.symbol.as_str(),
            trade.price,
            trade.quantity
        );
        
        // Get encoder and encode the data
        if let Some(encoder_ref) = self.encoders.get(&trade.symbol) {
            // Note: This is a simplified approach since the ARES encoder requires mutable access
            // In a real implementation, you'd want to use interior mutability or a different pattern
            let spikes = vec![]; // Placeholder - proper encoding would require encoder modification
            self.send_spikes(trade.symbol, spikes).await?;
        }
        
        Ok(())
    }
    
    /// Process quote and generate spikes
    async fn process_quote(&self, quote: crate::exchanges::UniversalQuote) -> Result<()> {
        self.initialize_symbol(quote.symbol.clone());
        
        // Update market state
        if let Some(tracker) = self.state_trackers.get(&quote.symbol) {
            let mut tracker = Arc::clone(&tracker.value());
            let tracker_mut = Arc::get_mut(&mut tracker).unwrap();
            tracker_mut.update_from_quote(
                quote.bid_price,
                quote.ask_price,
                quote.bid_size + quote.ask_size
            );
        }
        
        // Create market data for encoding (using mid price and total size)
        let mid_price = (quote.bid_price + quote.ask_price) / 2.0;
        let total_size = quote.bid_size + quote.ask_size;
        let market_data = MarketData::new(
            quote.symbol.as_str(),
            mid_price,
            total_size
        );
        
        // Get encoder and encode the data (placeholder implementation)
        if let Some(_encoder_ref) = self.encoders.get(&quote.symbol) {
            let spikes = vec![]; // Placeholder - proper encoding would require encoder modification
            self.send_spikes(quote.symbol, spikes).await?;
        }
        
        Ok(())
    }
    
    /// Process order book and generate spikes
    async fn process_orderbook(&self, book: crate::exchanges::UniversalOrderBook) -> Result<()> {
        self.initialize_symbol(book.symbol.clone());
        
        // Update market state
        if let Some(tracker) = self.state_trackers.get(&book.symbol) {
            if !book.bids.is_empty() && !book.asks.is_empty() {
                let mut tracker = Arc::clone(&tracker.value());
                let tracker_mut = Arc::get_mut(&mut tracker).unwrap();
                
                let best_bid = book.bids[0].0;
                let best_ask = book.asks[0].0;
                let total_volume = book.bids.iter().map(|(_, q)| q).sum::<f64>() +
                                 book.asks.iter().map(|(_, q)| q).sum::<f64>();
                
                tracker_mut.update_from_quote(best_bid, best_ask, total_volume);
            }
        }
        
        // Create market data for encoding (using best bid/ask)
        if !book.bids.is_empty() && !book.asks.is_empty() {
            let mid_price = (book.bids[0].0 + book.asks[0].0) / 2.0;
            let total_volume = book.bids.iter().map(|(_, q)| q).sum::<f64>() +
                             book.asks.iter().map(|(_, q)| q).sum::<f64>();
            
            let market_data = MarketData::new(
                book.symbol.as_str(),
                mid_price,
                total_volume
            );
            
            // Get encoder and encode the data (placeholder implementation)
            if let Some(_encoder_ref) = self.encoders.get(&book.symbol) {
                let spikes = vec![]; // Placeholder - proper encoding would require encoder modification
                self.send_spikes(book.symbol, spikes).await?;
            }
        }
        
        Ok(())
    }
    
    /// Send spikes and update statistics
    async fn send_spikes(&self, symbol: Symbol, spikes: Vec<Spike>) -> Result<()> {
        if spikes.is_empty() {
            return Ok(());
        }
        
        // Buffer spikes if batching is enabled
        if self.config.batch_size > 1 {
            let mut buffer = self.spike_buffer.get_mut(&symbol).unwrap();
            buffer.extend(spikes.clone());
            
            if buffer.len() >= self.config.batch_size {
                let batch = buffer.drain(..).collect::<Vec<_>>();
                self.spike_sender.send(batch)?;
            }
        } else {
            self.spike_sender.send(spikes.clone())?;
        }
        
        // Update statistics
        if let Some(mut stats) = self.statistics.get_mut(&symbol) {
            stats.events_processed += 1;
            stats.spikes_generated += spikes.len() as u64;
            
            if let Some(last_time) = stats.last_spike_time {
                let elapsed = last_time.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    stats.spike_rate_hz = spikes.len() as f64 / elapsed;
                }
            }
            stats.last_spike_time = Some(Instant::now());
        }
        
        Ok(())
    }
    
    /// Flush buffered spikes for a symbol
    pub async fn flush_symbol(&self, symbol: &Symbol) -> Result<()> {
        if let Some(mut buffer) = self.spike_buffer.get_mut(symbol) {
            if !buffer.is_empty() {
                let batch = buffer.drain(..).collect::<Vec<_>>();
                self.spike_sender.send(batch)?;
            }
        }
        Ok(())
    }
    
    /// Flush all buffered spikes
    pub async fn flush_all(&self) -> Result<()> {
        for entry in self.spike_buffer.iter() {
            let symbol = entry.key().clone();
            drop(entry); // Release the lock
            self.flush_symbol(&symbol).await?;
        }
        Ok(())
    }
    
    /// Subscribe to spike stream
    pub fn subscribe(&mut self) -> Option<mpsc::UnboundedReceiver<Vec<Spike>>> {
        self.spike_receiver.take()
    }
    
    /// Get statistics for a symbol
    pub fn get_statistics(&self, symbol: &Symbol) -> Option<SpikeStatistics> {
        self.statistics.get(symbol).map(|s| s.clone())
    }
    
    /// Get market state for a symbol
    pub fn get_market_state(&self, symbol: &Symbol) -> Option<MarketState> {
        self.state_trackers.get(symbol).map(|tracker| {
            let tracker = Arc::clone(&tracker.value());
            // This would need proper accessor methods in real implementation
            MarketState::default()
        })
    }
    
    /// Adaptive encoding adjustment based on market conditions
    pub fn adjust_encoding(&self, symbol: &Symbol) {
        if !self.config.enable_adaptive_encoding {
            return;
        }
        
        // Note: The ARES encoder doesn't have a set_sensitivity method
        // This would need to be implemented differently, possibly by creating
        // new encoders with different configurations based on market conditions
        if let Some(_tracker) = self.state_trackers.get(symbol) {
            if let Some(_encoder) = self.encoders.get(symbol) {
                let state = self.get_market_state(symbol).unwrap_or_default();
                
                // Placeholder for adaptive encoding logic
                // In a real implementation, you might:
                // 1. Create new encoder configurations based on volatility
                // 2. Replace the encoder in the map
                // 3. Or adjust encoding parameters through configuration
                let _volatility_adjustment = if state.volatility > 0.02 {
                    2.0 // High volatility
                } else if state.volatility > 0.01 {
                    1.5 // Medium volatility
                } else {
                    1.0 // Low volatility
                };
            }
        }
    }
}

/// Integration handler for connecting unified feed to spike bridge
pub struct MarketSpikeIntegration {
    feed: Arc<UnifiedMarketFeed>,
    bridge: Arc<MarketDataSpikeBridge>,
    running: Arc<tokio::sync::RwLock<bool>>,
}

impl MarketSpikeIntegration {
    pub fn new(
        mut feed: UnifiedMarketFeed,
        bridge: MarketDataSpikeBridge
    ) -> Self {
        Self {
            feed: Arc::new(feed),
            bridge: Arc::new(bridge),
            running: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }
    
    /// Start processing market events
    pub async fn start(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = true;
        drop(running);
        
        // Get event receiver from feed
        let mut feed = Arc::get_mut(&mut self.feed).unwrap();
        let mut receiver = feed.subscribe()
            .ok_or_else(|| anyhow::anyhow!("Failed to subscribe to feed"))?;
        
        let bridge = self.bridge.clone();
        let running = self.running.clone();
        
        // Spawn processing task
        tokio::spawn(async move {
            while *running.read().await {
                tokio::select! {
                    Some(event) = receiver.recv() => {
                        if let Err(e) = bridge.process_event(event).await {
                            eprintln!("Error processing event: {}", e);
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        // Periodic flush
                        if let Err(e) = bridge.flush_all().await {
                            eprintln!("Error flushing spikes: {}", e);
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop processing
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        
        // Final flush
        self.bridge.flush_all().await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_spike_bridge() {
        let config = SpikeBridgeConfig::default();
        let mut bridge = MarketDataSpikeBridge::new(config);
        
        let quote = crate::exchanges::UniversalQuote {
            exchange: Exchange::Binance,
            symbol: Symbol::new("BTC-USD"),
            bid_price: 50000.0,
            bid_size: 1.0,
            ask_price: 50001.0,
            ask_size: 1.0,
            timestamp_exchange: 0,
            timestamp_local: 0,
        };
        
        let event = UnifiedMarketEvent::Quote(quote);
        bridge.process_event(event).await.unwrap();
        
        let stats = bridge.get_statistics(&Symbol::new("BTC-USD"));
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().events_processed, 1);
    }
}