use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use super::{MarketData, ScannerConfig, MarketMetrics, MarketRegime};
use crate::exchanges::Symbol;

#[derive(Clone)]
pub struct MarketScanner {
    config: ScannerConfig,
    active_symbols: Arc<RwLock<Vec<Symbol>>>,
    market_cache: Arc<RwLock<HashMap<String, MarketData>>>,
    scan_stats: Arc<RwLock<ScanStats>>,
}

#[derive(Debug, Clone, Default)]
struct ScanStats {
    total_scans: u64,
    symbols_processed: u64,
    opportunities_found: u64,
    avg_scan_time_ms: f64,
    last_scan_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl MarketScanner {
    pub fn new(config: ScannerConfig) -> Self {
        Self {
            config,
            active_symbols: Arc::new(RwLock::new(Vec::new())),
            market_cache: Arc::new(RwLock::new(HashMap::new())),
            scan_stats: Arc::new(RwLock::new(ScanStats::default())),
        }
    }

    pub async fn start_scanning(&self) -> Result<()> {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_millis(self.config.scan_interval_ms)
        );

        loop {
            interval.tick().await;
            self.perform_scan().await?;
        }
    }

    pub async fn add_symbol(&self, symbol: Symbol) -> Result<()> {
        let mut symbols = self.active_symbols.write().await;
        if !symbols.contains(&symbol) && symbols.len() < self.config.max_symbols {
            symbols.push(symbol);
        }
        Ok(())
    }

    pub async fn add_symbols(&self, symbols: Vec<Symbol>) -> Result<()> {
        let mut active_symbols = self.active_symbols.write().await;
        for symbol in symbols {
            if !active_symbols.contains(&symbol) && active_symbols.len() < self.config.max_symbols {
                active_symbols.push(symbol);
            }
        }
        Ok(())
    }

    pub async fn remove_symbol(&self, symbol: &Symbol) -> Result<()> {
        let mut symbols = self.active_symbols.write().await;
        symbols.retain(|s| s != symbol);
        Ok(())
    }

    pub async fn get_active_symbols(&self) -> Vec<Symbol> {
        self.active_symbols.read().await.clone()
    }

    pub async fn get_market_data(&self, symbol: &str) -> Option<MarketData> {
        self.market_cache.read().await.get(symbol).cloned()
    }

    pub async fn update_market_data(&self, data: MarketData) -> Result<()> {
        let mut cache = self.market_cache.write().await;
        cache.insert(data.symbol.name.clone(), data);
        Ok(())
    }

    pub async fn get_scan_statistics(&self) -> ScanStats {
        self.scan_stats.read().await.clone()
    }

    pub async fn filter_by_criteria(&self, data: Vec<MarketData>) -> Result<Vec<MarketData>> {
        let mut filtered = Vec::new();

        for market_data in data {
            if self.meets_basic_criteria(&market_data) {
                filtered.push(market_data);
            }
        }

        Ok(filtered)
    }

    pub async fn rank_opportunities(&self, data: Vec<MarketData>) -> Result<Vec<(MarketData, f64)>> {
        let mut ranked = Vec::new();

        for market_data in data {
            let score = self.calculate_opportunity_score(&market_data).await;
            ranked.push((market_data, score));
        }

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        Ok(ranked)
    }

    async fn perform_scan(&self) -> Result<()> {
        let start_time = std::time::Instant::now();
        let symbols = self.active_symbols.read().await.clone();
        
        let mut stats = self.scan_stats.write().await;
        stats.total_scans += 1;
        stats.symbols_processed += symbols.len() as u64;
        stats.last_scan_timestamp = Some(chrono::Utc::now());
        
        let scan_duration = start_time.elapsed().as_millis() as f64;
        stats.avg_scan_time_ms = (stats.avg_scan_time_ms + scan_duration) / 2.0;

        Ok(())
    }

    fn meets_basic_criteria(&self, data: &MarketData) -> bool {
        if data.price < self.config.min_price_threshold {
            return false;
        }

        if data.price > self.config.max_price_threshold {
            return false;
        }

        if data.volume < self.config.min_volume_threshold {
            return false;
        }

        if data.change_24h.abs() < 0.5 {
            return false;
        }

        true
    }

    async fn calculate_opportunity_score(&self, data: &MarketData) -> f64 {
        let mut score = 0.0;

        score += self.volume_score(data) * 0.3;
        score += self.momentum_score(data) * 0.3;
        score += self.volatility_score(data) * 0.2;
        score += self.price_action_score(data) * 0.2;

        score.min(1.0).max(0.0)
    }

    fn volume_score(&self, data: &MarketData) -> f64 {
        let volume_ratio = data.volume / self.config.min_volume_threshold;
        match volume_ratio {
            x if x > 10.0 => 1.0,
            x if x > 5.0 => 0.8,
            x if x > 2.0 => 0.6,
            x if x > 1.0 => 0.4,
            _ => 0.2,
        }
    }

    fn momentum_score(&self, data: &MarketData) -> f64 {
        let change_abs = data.change_24h.abs();
        match change_abs {
            x if x > 10.0 => 1.0,
            x if x > 5.0 => 0.8,
            x if x > 2.0 => 0.6,
            x if x > 1.0 => 0.4,
            _ => 0.2,
        }
    }

    fn volatility_score(&self, data: &MarketData) -> f64 {
        let volatility = (data.high - data.low) / data.price;
        match volatility {
            x if x > 0.1 => 1.0,
            x if x > 0.05 => 0.8,
            x if x > 0.03 => 0.6,
            x if x > 0.01 => 0.4,
            _ => 0.2,
        }
    }

    fn price_action_score(&self, data: &MarketData) -> f64 {
        let close_position = (data.price - data.low) / (data.high - data.low);
        
        if data.change_24h > 0.0 && close_position > 0.7 {
            0.8
        } else if data.change_24h < 0.0 && close_position < 0.3 {
            0.8
        } else {
            0.4
        }
    }
}