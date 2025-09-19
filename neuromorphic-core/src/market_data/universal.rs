//! Universal market data types and traits

use crate::exchanges::{UniversalTrade, UniversalQuote, UniversalOrderBook};
use anyhow::Result;

/// Trait for normalizing exchange-specific data to universal format
pub trait MarketDataNormalizer: Send + Sync {
    fn normalize_trade(&self, raw: &[u8]) -> Result<UniversalTrade>;
    fn normalize_quote(&self, raw: &[u8]) -> Result<UniversalQuote>;
    fn normalize_book(&self, raw: &[u8]) -> Result<UniversalOrderBook>;
    fn exchange_name(&self) -> &str;
}