//! Symbol mapping between exchanges and universal format

use crate::exchanges::{Symbol, Exchange};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Symbol mapper for cross-exchange translation
pub struct SymbolMapper {
    universal_to_exchange: DashMap<(Symbol, Exchange), String>,
    exchange_to_universal: DashMap<(String, Exchange), Symbol>,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

impl SymbolMapper {
    pub fn new() -> Self {
        let mapper = Self {
            universal_to_exchange: DashMap::new(),
            exchange_to_universal: DashMap::new(),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        };
        
        // Preload common mappings
        mapper.preload_mappings();
        mapper
    }
    
    fn preload_mappings(&self) {
        // Bitcoin mappings
        self.add_mapping(Symbol::new("BTC-USD"), Exchange::Binance, "BTCUSDT");
        self.add_mapping(Symbol::new("BTC-USD"), Exchange::Coinbase, "BTC-USD");
        self.add_mapping(Symbol::new("BTC-USD"), Exchange::Kraken, "XXBTZUSD");
        
        // Ethereum mappings
        self.add_mapping(Symbol::new("ETH-USD"), Exchange::Binance, "ETHUSDT");
        self.add_mapping(Symbol::new("ETH-USD"), Exchange::Coinbase, "ETH-USD");
        self.add_mapping(Symbol::new("ETH-USD"), Exchange::Kraken, "XETHZUSD");
        
        // Other major pairs
        self.add_mapping(Symbol::new("BNB-USD"), Exchange::Binance, "BNBUSDT");
        self.add_mapping(Symbol::new("SOL-USD"), Exchange::Binance, "SOLUSDT");
        self.add_mapping(Symbol::new("SOL-USD"), Exchange::Coinbase, "SOL-USD");
        self.add_mapping(Symbol::new("XRP-USD"), Exchange::Binance, "XRPUSDT");
        self.add_mapping(Symbol::new("XRP-USD"), Exchange::Coinbase, "XRP-USD");
        self.add_mapping(Symbol::new("ADA-USD"), Exchange::Binance, "ADAUSDT");
        self.add_mapping(Symbol::new("ADA-USD"), Exchange::Coinbase, "ADA-USD");
        self.add_mapping(Symbol::new("AVAX-USD"), Exchange::Binance, "AVAXUSDT");
        self.add_mapping(Symbol::new("DOGE-USD"), Exchange::Binance, "DOGEUSDT");
        self.add_mapping(Symbol::new("DOT-USD"), Exchange::Binance, "DOTUSDT");
        self.add_mapping(Symbol::new("MATIC-USD"), Exchange::Binance, "MATICUSDT");
        
        // Cross pairs
        self.add_mapping(Symbol::new("ETH-BTC"), Exchange::Binance, "ETHBTC");
        self.add_mapping(Symbol::new("ETH-BTC"), Exchange::Coinbase, "ETH-BTC");
        
        // Stablecoin pairs
        self.add_mapping(Symbol::new("BTC-BUSD"), Exchange::Binance, "BTCBUSD");
        self.add_mapping(Symbol::new("ETH-BUSD"), Exchange::Binance, "ETHBUSD");
    }
    
    pub fn add_mapping(&self, universal: Symbol, exchange: Exchange, exchange_symbol: &str) {
        self.universal_to_exchange.insert(
            (universal.clone(), exchange),
            exchange_symbol.to_string()
        );
        self.exchange_to_universal.insert(
            (exchange_symbol.to_string(), exchange),
            universal
        );
    }
    
    pub fn to_exchange(&self, symbol: &Symbol, exchange: Exchange) -> Option<String> {
        let result = self.universal_to_exchange
            .get(&(symbol.clone(), exchange))
            .map(|e| e.clone());
        
        if result.is_some() {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
        
        result
    }
    
    pub fn from_exchange(&self, exchange_symbol: &str, exchange: Exchange) -> Option<Symbol> {
        let result = self.exchange_to_universal
            .get(&(exchange_symbol.to_string(), exchange))
            .map(|e| e.clone());
        
        if result.is_some() {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
            
            // Try to learn the mapping dynamically
            if let Some(universal) = self.infer_universal_symbol(exchange_symbol, exchange) {
                self.add_mapping(universal.clone(), exchange, exchange_symbol);
                return Some(universal);
            }
        }
        
        result
    }
    
    fn infer_universal_symbol(&self, exchange_symbol: &str, exchange: Exchange) -> Option<Symbol> {
        match exchange {
            Exchange::Binance => {
                // Binance uses BASEQOUTE format (e.g., BTCUSDT)
                if exchange_symbol.ends_with("USDT") {
                    let base = &exchange_symbol[..exchange_symbol.len() - 4];
                    Some(Symbol::new(format!("{}-USD", base)))
                } else if exchange_symbol.ends_with("BUSD") {
                    let base = &exchange_symbol[..exchange_symbol.len() - 4];
                    Some(Symbol::new(format!("{}-BUSD", base)))
                } else if exchange_symbol.ends_with("BTC") {
                    let base = &exchange_symbol[..exchange_symbol.len() - 3];
                    Some(Symbol::new(format!("{}-BTC", base)))
                } else {
                    None
                }
            }
            Exchange::Coinbase => {
                // Coinbase uses BASE-QUOTE format
                Some(Symbol::new(exchange_symbol))
            }
            Exchange::Kraken => {
                // Kraken uses weird prefixes (XBT for BTC, etc.)
                let cleaned = exchange_symbol
                    .replace("XXBT", "BTC")
                    .replace("XETH", "ETH")
                    .replace("ZUSD", "-USD");
                Some(Symbol::new(cleaned))
            }
            _ => None,
        }
    }
    
    pub fn get_cache_stats(&self) -> (u64, u64, f64) {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };
        (hits, misses, hit_rate)
    }
    
    pub fn clear_cache_stats(&self) {
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
    }
}