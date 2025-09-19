//! Exchange-specific normalizers

use super::{MarketDataNormalizer, SymbolMapper, TimeSynchronizer};
use crate::exchanges::{
    UniversalTrade, UniversalQuote, UniversalOrderBook, 
    Exchange, Symbol, Side, BinanceTrade
};
use anyhow::Result;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Binance data normalizer
pub struct BinanceNormalizer {
    symbol_mapper: Arc<SymbolMapper>,
    time_sync: Arc<TimeSynchronizer>,
}

impl BinanceNormalizer {
    pub fn new(symbol_mapper: Arc<SymbolMapper>, time_sync: Arc<TimeSynchronizer>) -> Self {
        Self {
            symbol_mapper,
            time_sync,
        }
    }
}

impl MarketDataNormalizer for BinanceNormalizer {
    fn normalize_trade(&self, raw: &[u8]) -> Result<UniversalTrade> {
        let binance_trade: BinanceTrade = serde_json::from_slice(raw)?;
        
        let symbol = self.symbol_mapper
            .from_exchange(&binance_trade.symbol, Exchange::Binance)
            .unwrap_or_else(|| Symbol::new(binance_trade.symbol.clone()));
        
        let local_time = self.time_sync.adjust_timestamp(
            binance_trade.trade_time,
            Exchange::Binance
        );
        
        Ok(UniversalTrade {
            exchange: Exchange::Binance,
            symbol,
            price: binance_trade.price.parse()?,
            quantity: binance_trade.quantity.parse()?,
            side: if binance_trade.is_buyer_maker { 
                Side::Sell 
            } else { 
                Side::Buy 
            },
            timestamp_exchange: binance_trade.trade_time,
            timestamp_local: local_time,
            trade_id: format!("BIN_{}", binance_trade.trade_id),
        })
    }
    
    fn normalize_quote(&self, raw: &[u8]) -> Result<UniversalQuote> {
        // Parse Binance quote format
        let quote: serde_json::Value = serde_json::from_slice(raw)?;
        
        let symbol_str = quote["s"].as_str().unwrap_or("UNKNOWN");
        let symbol = self.symbol_mapper
            .from_exchange(symbol_str, Exchange::Binance)
            .unwrap_or_else(|| Symbol::new(symbol_str));
        
        Ok(UniversalQuote {
            exchange: Exchange::Binance,
            symbol,
            bid_price: quote["b"].as_str().unwrap_or("0").parse()?,
            bid_size: quote["B"].as_str().unwrap_or("0").parse()?,
            ask_price: quote["a"].as_str().unwrap_or("0").parse()?,
            ask_size: quote["A"].as_str().unwrap_or("0").parse()?,
            timestamp_exchange: quote["E"].as_u64().unwrap_or(0),
            timestamp_local: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_millis() as u64,
        })
    }
    
    fn normalize_book(&self, raw: &[u8]) -> Result<UniversalOrderBook> {
        let book: serde_json::Value = serde_json::from_slice(raw)?;
        
        let symbol_str = book["s"].as_str().unwrap_or("UNKNOWN");
        let symbol = self.symbol_mapper
            .from_exchange(symbol_str, Exchange::Binance)
            .unwrap_or_else(|| Symbol::new(symbol_str));
        
        let mut bids = Vec::new();
        if let Some(bid_array) = book["b"].as_array() {
            for bid in bid_array.iter().take(10) {
                if let Some(arr) = bid.as_array() {
                    if arr.len() >= 2 {
                        let price: f64 = arr[0].as_str().unwrap_or("0").parse()?;
                        let qty: f64 = arr[1].as_str().unwrap_or("0").parse()?;
                        bids.push((price, qty));
                    }
                }
            }
        }
        
        let mut asks = Vec::new();
        if let Some(ask_array) = book["a"].as_array() {
            for ask in ask_array.iter().take(10) {
                if let Some(arr) = ask.as_array() {
                    if arr.len() >= 2 {
                        let price: f64 = arr[0].as_str().unwrap_or("0").parse()?;
                        let qty: f64 = arr[1].as_str().unwrap_or("0").parse()?;
                        asks.push((price, qty));
                    }
                }
            }
        }
        
        Ok(UniversalOrderBook {
            exchange: Exchange::Binance,
            symbol,
            bids,
            asks,
            timestamp_exchange: book["E"].as_u64().unwrap_or(0),
            timestamp_local: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_millis() as u64,
            sequence: book["u"].as_u64().unwrap_or(0),
        })
    }
    
    fn exchange_name(&self) -> &str {
        "Binance"
    }
}

/// Coinbase data normalizer
pub struct CoinbaseNormalizer {
    symbol_mapper: Arc<SymbolMapper>,
    time_sync: Arc<TimeSynchronizer>,
}

impl CoinbaseNormalizer {
    pub fn new(symbol_mapper: Arc<SymbolMapper>, time_sync: Arc<TimeSynchronizer>) -> Self {
        Self {
            symbol_mapper,
            time_sync,
        }
    }
}

impl MarketDataNormalizer for CoinbaseNormalizer {
    fn normalize_trade(&self, raw: &[u8]) -> Result<UniversalTrade> {
        let trade: serde_json::Value = serde_json::from_slice(raw)?;
        
        let symbol_str = trade["product_id"].as_str().unwrap_or("UNKNOWN");
        let symbol = self.symbol_mapper
            .from_exchange(symbol_str, Exchange::Coinbase)
            .unwrap_or_else(|| Symbol::new(symbol_str));
        
        Ok(UniversalTrade {
            exchange: Exchange::Coinbase,
            symbol,
            price: trade["price"].as_str().unwrap_or("0").parse()?,
            quantity: trade["size"].as_str().unwrap_or("0").parse()?,
            side: match trade["side"].as_str() {
                Some("buy") => Side::Buy,
                _ => Side::Sell,
            },
            timestamp_exchange: 0, // Parse from trade["time"]
            timestamp_local: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_millis() as u64,
            trade_id: trade["trade_id"].to_string(),
        })
    }
    
    fn normalize_quote(&self, raw: &[u8]) -> Result<UniversalQuote> {
        let quote: serde_json::Value = serde_json::from_slice(raw)?;
        
        let symbol_str = quote["product_id"].as_str().unwrap_or("UNKNOWN");
        let symbol = self.symbol_mapper
            .from_exchange(symbol_str, Exchange::Coinbase)
            .unwrap_or_else(|| Symbol::new(symbol_str));
        
        Ok(UniversalQuote {
            exchange: Exchange::Coinbase,
            symbol,
            bid_price: quote["best_bid"].as_str().unwrap_or("0").parse()?,
            bid_size: quote["best_bid_size"].as_str().unwrap_or("0").parse()?,
            ask_price: quote["best_ask"].as_str().unwrap_or("0").parse()?,
            ask_size: quote["best_ask_size"].as_str().unwrap_or("0").parse()?,
            timestamp_exchange: 0,
            timestamp_local: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_millis() as u64,
        })
    }
    
    fn normalize_book(&self, raw: &[u8]) -> Result<UniversalOrderBook> {
        let book: serde_json::Value = serde_json::from_slice(raw)?;
        
        let symbol_str = book["product_id"].as_str().unwrap_or("UNKNOWN");
        let symbol = self.symbol_mapper
            .from_exchange(symbol_str, Exchange::Coinbase)
            .unwrap_or_else(|| Symbol::new(symbol_str));
        
        let mut bids = Vec::new();
        if let Some(bid_array) = book["bids"].as_array() {
            for bid in bid_array.iter().take(10) {
                if let Some(arr) = bid.as_array() {
                    if arr.len() >= 2 {
                        let price: f64 = arr[0].as_str().unwrap_or("0").parse()?;
                        let qty: f64 = arr[1].as_str().unwrap_or("0").parse()?;
                        bids.push((price, qty));
                    }
                }
            }
        }
        
        let mut asks = Vec::new();
        if let Some(ask_array) = book["asks"].as_array() {
            for ask in ask_array.iter().take(10) {
                if let Some(arr) = ask.as_array() {
                    if arr.len() >= 2 {
                        let price: f64 = arr[0].as_str().unwrap_or("0").parse()?;
                        let qty: f64 = arr[1].as_str().unwrap_or("0").parse()?;
                        asks.push((price, qty));
                    }
                }
            }
        }
        
        Ok(UniversalOrderBook {
            exchange: Exchange::Coinbase,
            symbol,
            bids,
            asks,
            timestamp_exchange: 0,
            timestamp_local: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_millis() as u64,
            sequence: book["sequence"].as_u64().unwrap_or(0),
        })
    }
    
    fn exchange_name(&self) -> &str {
        "Coinbase"
    }
}