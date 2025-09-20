use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use chrono::{DateTime, Utc};
use crate::exchanges::{Symbol, Exchange};

pub mod scanner;
pub mod screener;
pub mod strategies;
pub mod analytics;
pub mod data_feeds;

pub use scanner::MarketScanner;
pub use screener::{StockScreener, ScreeningCriteria};
pub use strategies::{StrategyEngine, TradingStrategy};
pub use analytics::MarketAnalytics;
pub use data_feeds::{DataFeedManager, MarketDataFeed};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub symbol: Symbol,
    pub price: f64,
    pub volume: f64,
    pub timestamp: DateTime<Utc>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub change_24h: f64,
    pub volume_24h: f64,
}

impl MarketData {
    pub fn new(symbol: Symbol, price: f64) -> Self {
        let now = Utc::now();
        Self {
            symbol,
            price,
            volume: 0.0,
            timestamp: now,
            bid: None,
            ask: None,
            open: price,
            high: price,
            low: price,
            change_24h: 0.0,
            volume_24h: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingOpportunity {
    pub symbol: Symbol,
    pub strategy: String,
    pub confidence: f64,
    pub expected_move: f64,
    pub time_horizon: String,
    pub entry_price: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub position_size: f64,
    pub reasoning: String,
    pub risk_score: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMetrics {
    pub total_symbols_tracked: usize,
    pub opportunities_detected: usize,
    pub market_volatility: f64,
    pub sector_performance: HashMap<String, f64>,
    pub trending_symbols: Vec<Symbol>,
    pub market_regime: MarketRegime,
    pub overall_sentiment: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketRegime {
    StrongBull,
    MildBull,
    Consolidation,
    MildBear,
    StrongBear,
    HighVolatility,
    LowVolatility,
}

#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub max_symbols: usize,
    pub scan_interval_ms: u64,
    pub min_volume_threshold: f64,
    pub min_price_threshold: f64,
    pub max_price_threshold: f64,
    pub excluded_sectors: Vec<String>,
    pub included_exchanges: Vec<Exchange>,
    pub enable_premarket: bool,
    pub enable_afterhours: bool,
    pub momentum_lookback_periods: Vec<usize>,
    pub volatility_threshold: f64,
    pub volume_spike_threshold: f64,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            max_symbols: 5000,
            scan_interval_ms: 1000,
            min_volume_threshold: 100000.0,
            min_price_threshold: 1.0,
            max_price_threshold: 1000.0,
            excluded_sectors: vec!["Penny Stocks".to_string()],
            included_exchanges: vec![Exchange::NYSE, Exchange::NASDAQ],
            enable_premarket: true,
            enable_afterhours: true,
            momentum_lookback_periods: vec![5, 15, 30, 60],
            volatility_threshold: 2.0,
            volume_spike_threshold: 3.0,
        }
    }
}

pub type MarketDataStream = broadcast::Receiver<MarketData>;
pub type OpportunityStream = broadcast::Receiver<TradingOpportunity>;

#[derive(Clone)]
pub struct MarketScannerService {
    scanner: Arc<MarketScanner>,
    screener: Arc<StockScreener>,
    strategy_engine: Arc<StrategyEngine>,
    data_feeds: Arc<DataFeedManager>,
    market_data: Arc<RwLock<HashMap<Symbol, MarketData>>>,
    config: ScannerConfig,
}

impl MarketScannerService {
    pub fn new(config: ScannerConfig) -> Self {
        let scanner = Arc::new(MarketScanner::new(config.clone()));
        let screener = Arc::new(StockScreener::new());
        let strategy_engine = Arc::new(StrategyEngine::new());
        let data_feeds = Arc::new(DataFeedManager::new(config.clone()));
        let market_data = Arc::new(RwLock::new(HashMap::new()));

        Self {
            scanner,
            screener,
            strategy_engine,
            data_feeds,
            market_data,
            config,
        }
    }

    pub async fn start(&self) -> Result<(MarketDataStream, OpportunityStream)> {
        let (market_tx, market_rx) = broadcast::channel(10000);
        let (opportunity_tx, opportunity_rx) = broadcast::channel(1000);

        let data_feeds = self.data_feeds.clone();
        let scanner = self.scanner.clone();
        let screener = self.screener.clone();
        let strategy_engine = self.strategy_engine.clone();
        let market_data = self.market_data.clone();

        tokio::spawn(async move {
            let mut data_stream = data_feeds.start_all_feeds().await.unwrap();
            
            loop {
                tokio::select! {
                    Some(market_update) = data_stream.recv() => {
                        {
                            let mut data = market_data.write().await;
                            data.insert(market_update.symbol.clone(), market_update.clone());
                        }
                        
                        let _ = market_tx.send(market_update.clone());
                        
                        if let Ok(opportunities) = strategy_engine.analyze_opportunity(&market_update).await {
                            for opportunity in opportunities {
                                let _ = opportunity_tx.send(opportunity);
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)) => {
                        let data = market_data.read().await;
                        if let Ok(filtered_symbols) = screener.screen_symbols(data.values().cloned().collect()).await {
                            for symbol_data in filtered_symbols {
                                if let Ok(opportunities) = strategy_engine.analyze_opportunity(&symbol_data).await {
                                    for opportunity in opportunities {
                                        let _ = opportunity_tx.send(opportunity);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok((market_rx, opportunity_rx))
    }

    pub async fn get_market_metrics(&self) -> Result<MarketMetrics> {
        let data = self.market_data.read().await;
        let analytics = MarketAnalytics::new();
        analytics.calculate_market_metrics(data.values().cloned().collect()).await
    }

    pub async fn get_top_opportunities(&self, limit: usize) -> Result<Vec<TradingOpportunity>> {
        let data = self.market_data.read().await;
        let mut all_opportunities = Vec::new();
        
        for market_data in data.values() {
            if let Ok(opportunities) = self.strategy_engine.analyze_opportunity(market_data).await {
                all_opportunities.extend(opportunities);
            }
        }
        
        all_opportunities.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        all_opportunities.truncate(limit);
        
        Ok(all_opportunities)
    }
}