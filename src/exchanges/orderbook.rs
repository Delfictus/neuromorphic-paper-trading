//! Order book management

use super::{Symbol, ExchangeError};
use anyhow::Result;
use ordered_float::OrderedFloat;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Order book depth update
#[derive(Deserialize, Debug)]
pub struct DepthUpdate {
    #[serde(rename = "U")]
    pub first_update_id: u64,
    #[serde(rename = "u")]
    pub final_update_id: u64,
    #[serde(rename = "b")]
    pub bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    pub asks: Vec<[String; 2]>,
    #[serde(rename = "E")]
    pub event_time: Option<u64>,
}

/// Order book snapshot response
#[derive(Deserialize, Debug)]
pub struct DepthSnapshot {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

/// Order book structure
#[derive(Clone, Debug)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
    pub last_update_id: u64,
    pub timestamp: u64,
}

impl OrderBook {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update_id: 0,
            timestamp: 0,
        }
    }
    
    /// Fetch order book snapshot from REST API
    pub async fn fetch_snapshot(symbol: &str) -> Result<Self> {
        let url = format!(
            "https://api.binance.com/api/v3/depth?symbol={}&limit=1000",
            symbol.to_uppercase()
        );
        
        let response = reqwest::get(&url).await?;
        let snapshot: DepthSnapshot = response.json().await?;
        
        let mut book = Self::new(symbol.to_string());
        
        // Parse bids
        for [price_str, qty_str] in snapshot.bids {
            let price = price_str.parse::<f64>()?;
            let qty = qty_str.parse::<f64>()?;
            book.bids.insert(OrderedFloat(price), qty);
        }
        
        // Parse asks
        for [price_str, qty_str] in snapshot.asks {
            let price = price_str.parse::<f64>()?;
            let qty = qty_str.parse::<f64>()?;
            book.asks.insert(OrderedFloat(price), qty);
        }
        
        book.last_update_id = snapshot.last_update_id;
        book.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis() as u64;
        
        Ok(book)
    }
    
    /// Apply depth update to order book
    pub fn apply_update(&mut self, update: DepthUpdate) -> Result<(), ExchangeError> {
        // Verify sequence
        if update.first_update_id > self.last_update_id + 1 {
            return Err(ExchangeError::SequenceGap {
                expected: self.last_update_id + 1,
                actual: update.first_update_id,
            });
        }
        
        // Apply bid updates
        for [price_str, qty_str] in update.bids {
            let price = price_str.parse::<f64>()
                .map_err(|e| ExchangeError::Parse(e.to_string()))?;
            let qty = qty_str.parse::<f64>()
                .map_err(|e| ExchangeError::Parse(e.to_string()))?;
            
            if qty == 0.0 {
                self.bids.remove(&OrderedFloat(price));
            } else {
                self.bids.insert(OrderedFloat(price), qty);
            }
        }
        
        // Apply ask updates
        for [price_str, qty_str] in update.asks {
            let price = price_str.parse::<f64>()
                .map_err(|e| ExchangeError::Parse(e.to_string()))?;
            let qty = qty_str.parse::<f64>()
                .map_err(|e| ExchangeError::Parse(e.to_string()))?;
            
            if qty == 0.0 {
                self.asks.remove(&OrderedFloat(price));
            } else {
                self.asks.insert(OrderedFloat(price), qty);
            }
        }
        
        self.last_update_id = update.final_update_id;
        self.timestamp = update.event_time.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        });
        
        Ok(())
    }
    
    /// Get mid price
    pub fn mid_price(&self) -> Option<f64> {
        let best_bid = self.bids.keys().next_back()?.0;
        let best_ask = self.asks.keys().next()?.0;
        Some((best_bid + best_ask) / 2.0)
    }
    
    /// Get spread in basis points
    pub fn spread_bps(&self) -> Option<f64> {
        let best_bid = self.bids.keys().next_back()?.0;
        let best_ask = self.asks.keys().next()?.0;
        let mid = (best_bid + best_ask) / 2.0;
        Some(((best_ask - best_bid) / mid) * 10000.0)
    }
    
    /// Calculate liquidity within percentage of mid price
    pub fn liquidity_within(&self, percentage: f64) -> (f64, f64) {
        let mid = match self.mid_price() {
            Some(m) => m,
            None => return (0.0, 0.0),
        };
        
        let lower_bound = mid * (1.0 - percentage);
        let upper_bound = mid * (1.0 + percentage);
        
        // Sum bid liquidity
        let bid_liquidity: f64 = self.bids
            .range(OrderedFloat(lower_bound)..)
            .map(|(price, qty)| price.0 * qty)
            .sum();
        
        // Sum ask liquidity
        let ask_liquidity: f64 = self.asks
            .range(..=OrderedFloat(upper_bound))
            .map(|(price, qty)| price.0 * qty)
            .sum();
        
        (bid_liquidity, ask_liquidity)
    }
    
    /// Verify order book integrity
    pub fn verify_integrity(&self) -> bool {
        if let (Some(best_bid), Some(best_ask)) = 
            (self.bids.keys().next_back(), self.asks.keys().next()) {
            // Best bid must be less than best ask
            if best_bid.0 >= best_ask.0 {
                return false;
            }
        }
        
        // Check for negative quantities (shouldn't happen but verify)
        for qty in self.bids.values() {
            if *qty < 0.0 {
                return false;
            }
        }
        
        for qty in self.asks.values() {
            if *qty < 0.0 {
                return false;
            }
        }
        
        true
    }
    
    /// Get best bid price and size
    pub fn best_bid(&self) -> Option<(f64, f64)> {
        self.bids.iter().next_back()
            .map(|(price, qty)| (price.0, *qty))
    }
    
    /// Get best ask price and size
    pub fn best_ask(&self) -> Option<(f64, f64)> {
        self.asks.iter().next()
            .map(|(price, qty)| (price.0, *qty))
    }
    
    /// Get top N levels
    pub fn top_levels(&self, n: usize) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let bids: Vec<(f64, f64)> = self.bids
            .iter()
            .rev()
            .take(n)
            .map(|(p, q)| (p.0, *q))
            .collect();
        
        let asks: Vec<(f64, f64)> = self.asks
            .iter()
            .take(n)
            .map(|(p, q)| (p.0, *q))
            .collect();
        
        (bids, asks)
    }
}