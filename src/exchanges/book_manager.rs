//! Multi-book manager for handling multiple order books

use super::{OrderBook, DepthUpdate, Symbol, Side, ExchangeError};
use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Arbitrage opportunity
#[derive(Clone, Debug)]
pub struct ArbitrageOpportunity {
    pub symbol_pair: (String, String),
    pub profit_bps: f64,
    pub side: Side,
    pub size: f64,
    pub exchange_buy: String,
    pub exchange_sell: String,
    pub detected_at: Instant,
}

/// Order book manager for multiple symbols
pub struct OrderBookManager {
    books: DashMap<String, Arc<RwLock<OrderBook>>>,
    update_counts: DashMap<String, AtomicU64>,
    last_arb_check: Arc<RwLock<Instant>>,
    total_updates: AtomicU64,
}

impl OrderBookManager {
    pub fn new() -> Self {
        Self {
            books: DashMap::new(),
            update_counts: DashMap::new(),
            last_arb_check: Arc::new(RwLock::new(Instant::now())),
            total_updates: AtomicU64::new(0),
        }
    }
    
    /// Initialize with multiple symbols
    pub async fn initialize(symbols: Vec<&str>) -> Result<Self> {
        let manager = Self::new();
        
        // Fetch all snapshots in parallel
        let futures: Vec<_> = symbols
            .iter()
            .map(|s| OrderBook::fetch_snapshot(s))
            .collect();
        
        let snapshots = futures::future::join_all(futures).await;
        
        for (symbol, snapshot_result) in symbols.iter().zip(snapshots) {
            match snapshot_result {
                Ok(book) => {
                    manager.books.insert(
                        symbol.to_string(),
                        Arc::new(RwLock::new(book))
                    );
                    manager.update_counts.insert(
                        symbol.to_string(),
                        AtomicU64::new(0)
                    );
                    println!("Initialized order book for {}", symbol);
                }
                Err(e) => {
                    eprintln!("Failed to initialize book for {}: {}", symbol, e);
                }
            }
        }
        
        Ok(manager)
    }
    
    /// Process order book update
    pub fn process_update(&self, symbol: String, update: DepthUpdate) -> Result<()> {
        let start = Instant::now();
        
        if let Some(book_ref) = self.books.get(&symbol) {
            let mut book = book_ref.write();
            book.apply_update(update)?;
            
            if let Some(counter) = self.update_counts.get(&symbol) {
                counter.fetch_add(1, Ordering::Relaxed);
            }
            
            self.total_updates.fetch_add(1, Ordering::Relaxed);
            
            // Track update latency
            let latency = start.elapsed();
            if latency.as_micros() > 100 {
                println!("Warning: Slow update for {}: {:?}", symbol, latency);
            }
        } else {
            return Err(anyhow::anyhow!("Unknown symbol: {}", symbol));
        }
        
        Ok(())
    }
    
    /// Find direct arbitrage opportunities (e.g., BTCUSDT vs BTCBUSD)
    pub fn find_direct_arbitrage(&self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        
        // Check BTC/USDT vs BTC/BUSD
        if let (Some(btcusdt), Some(btcbusd)) = 
            (self.books.get("BTCUSDT"), self.books.get("BTCBUSD")) {
            
            let book1 = btcusdt.read();
            let book2 = btcbusd.read();
            
            if let (Some((bid1, _)), Some((ask1, _)), Some((bid2, _)), Some((ask2, _))) = 
                (book1.best_bid(), book1.best_ask(), book2.best_bid(), book2.best_ask()) {
                
                // Check if we can buy on one and sell on the other
                let profit1 = (bid2 - ask1) / ask1 * 10000.0; // in bps
                let profit2 = (bid1 - ask2) / ask2 * 10000.0; // in bps
                
                if profit1 > 1.0 {  // 1 bps threshold
                    opportunities.push(ArbitrageOpportunity {
                        symbol_pair: ("BTCUSDT".to_string(), "BTCBUSD".to_string()),
                        profit_bps: profit1,
                        side: Side::Buy,
                        size: 0.1,  // Default size
                        exchange_buy: "BTCUSDT".to_string(),
                        exchange_sell: "BTCBUSD".to_string(),
                        detected_at: Instant::now(),
                    });
                }
                
                if profit2 > 1.0 {
                    opportunities.push(ArbitrageOpportunity {
                        symbol_pair: ("BTCBUSD".to_string(), "BTCUSDT".to_string()),
                        profit_bps: profit2,
                        side: Side::Buy,
                        size: 0.1,
                        exchange_buy: "BTCBUSD".to_string(),
                        exchange_sell: "BTCUSDT".to_string(),
                        detected_at: Instant::now(),
                    });
                }
            }
        }
        
        opportunities
    }
    
    /// Find triangular arbitrage opportunities
    pub fn find_triangular_arbitrage(&self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        
        // Check BTC -> ETH -> USDT -> BTC
        if let (Some(btcusdt), Some(ethusdt), Some(ethbtc)) = 
            (self.books.get("BTCUSDT"), self.books.get("ETHUSDT"), self.books.get("ETHBTC")) {
            
            let btc_book = btcusdt.read();
            let eth_book = ethusdt.read();
            let ethbtc_book = ethbtc.read();
            
            if let (Some((btc_bid, _)), Some((btc_ask, _)), 
                    Some((eth_bid, _)), Some((eth_ask, _)),
                    Some((ethbtc_bid, _)), Some((ethbtc_ask, _))) = 
                (btc_book.best_bid(), btc_book.best_ask(),
                 eth_book.best_bid(), eth_book.best_ask(),
                 ethbtc_book.best_bid(), ethbtc_book.best_ask()) {
                
                // Path 1: USD -> BTC -> ETH -> USD
                let btc_amount = 1000.0 / btc_ask;  // Buy BTC with $1000
                let eth_amount = btc_amount / ethbtc_ask;  // Convert BTC to ETH
                let usd_final = eth_amount * eth_bid;  // Sell ETH for USD
                let profit1 = (usd_final - 1000.0) / 1000.0 * 10000.0;  // in bps
                
                // Path 2: USD -> ETH -> BTC -> USD
                let eth_amount2 = 1000.0 / eth_ask;  // Buy ETH with $1000
                let btc_amount2 = eth_amount2 * ethbtc_bid;  // Convert ETH to BTC
                let usd_final2 = btc_amount2 * btc_bid;  // Sell BTC for USD
                let profit2 = (usd_final2 - 1000.0) / 1000.0 * 10000.0;  // in bps
                
                if profit1 > 1.0 {
                    opportunities.push(ArbitrageOpportunity {
                        symbol_pair: ("BTC-ETH-USD".to_string(), "Triangular".to_string()),
                        profit_bps: profit1,
                        side: Side::Buy,
                        size: 1000.0,
                        exchange_buy: "Path1".to_string(),
                        exchange_sell: "Triangular".to_string(),
                        detected_at: Instant::now(),
                    });
                }
                
                if profit2 > 1.0 {
                    opportunities.push(ArbitrageOpportunity {
                        symbol_pair: ("ETH-BTC-USD".to_string(), "Triangular".to_string()),
                        profit_bps: profit2,
                        side: Side::Buy,
                        size: 1000.0,
                        exchange_buy: "Path2".to_string(),
                        exchange_sell: "Triangular".to_string(),
                        detected_at: Instant::now(),
                    });
                }
            }
        }
        
        opportunities
    }
    
    /// Get order book for a symbol
    pub fn get_book(&self, symbol: &str) -> Option<Arc<RwLock<OrderBook>>> {
        self.books.get(symbol).map(|b| b.clone())
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> String {
        let mut stats = String::new();
        stats.push_str(&format!("Total updates: {}\n", self.total_updates.load(Ordering::Relaxed)));
        stats.push_str("Books:\n");
        
        for entry in self.books.iter() {
            let symbol = entry.key();
            let book = entry.value().read();
            let updates = self.update_counts
                .get(symbol)
                .map(|c| c.load(Ordering::Relaxed))
                .unwrap_or(0);
            
            stats.push_str(&format!(
                "  {}: {} bids, {} asks, {} updates\n",
                symbol,
                book.bids.len(),
                book.asks.len(),
                updates
            ));
        }
        
        stats
    }
    
    /// Check all arbitrage opportunities
    pub fn find_all_arbitrage(&self) -> Vec<ArbitrageOpportunity> {
        let mut all_opportunities = Vec::new();
        
        // Check if enough time has passed since last check
        let mut last_check = self.last_arb_check.write();
        if last_check.elapsed().as_millis() < 100 {
            return all_opportunities;  // Don't check too frequently
        }
        *last_check = Instant::now();
        
        // Find direct arbitrage
        all_opportunities.extend(self.find_direct_arbitrage());
        
        // Find triangular arbitrage
        all_opportunities.extend(self.find_triangular_arbitrage());
        
        // Sort by profit
        all_opportunities.sort_by(|a, b| {
            b.profit_bps.partial_cmp(&a.profit_bps).unwrap()
        });
        
        all_opportunities
    }
}