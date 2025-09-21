//! Metrics collection and API for Grafana integration
//! 
//! This module provides real-time metrics for the neuromorphic trading system
//! that can be consumed by Grafana dashboards.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::exchanges::Symbol;
use crate::paper_trading::{PositionStatistics, TradingSignal};

/// Real-time portfolio metrics for Grafana
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioMetrics {
    pub timestamp: DateTime<Utc>,
    pub total_capital: f64,
    pub available_capital: f64,
    pub total_pnl: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub total_return_pct: f64,
    pub positions_count: usize,
    pub active_positions_count: usize,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: f64,
}

/// Neuromorphic signal metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMetrics {
    pub timestamp: DateTime<Utc>,
    pub signals_processed: u64,
    pub signals_per_minute: f64,
    pub avg_confidence: f64,
    pub avg_urgency: f64,
    pub signal_distribution: HashMap<String, u64>, // Buy, Sell, Hold, Close counts
    pub pattern_strength_avg: f64,
    pub spike_count_avg: f64,
    pub volatility_avg: f64,
    pub market_regimes: HashMap<String, u64>, // uptrend, downtrend, consolidation counts
}

/// Position-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionMetrics {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub position_id: String,
    pub size: f64,
    pub entry_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_pct: f64,
    pub duration_minutes: i64,
    pub is_long: bool,
}

/// Market data metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMetrics {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub price: f64,
    pub volume_24h: f64,
    pub price_change_24h: f64,
    pub price_change_pct_24h: f64,
    pub volatility: f64,
    pub last_update: DateTime<Utc>,
}

/// Risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub timestamp: DateTime<Utc>,
    pub portfolio_var_95: f64,      // Value at Risk 95%
    pub portfolio_var_99: f64,      // Value at Risk 99%
    pub max_position_size_pct: f64,
    pub current_leverage: f64,
    pub correlation_btc: f64,
    pub correlation_eth: f64,
    pub concentration_risk: f64,    // Largest position as % of portfolio
    pub daily_volatility: f64,
}

/// Comprehensive metrics container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingMetrics {
    pub portfolio: PortfolioMetrics,
    pub signals: SignalMetrics,
    pub positions: Vec<PositionMetrics>,
    pub market_data: Vec<MarketMetrics>,
    pub risk: RiskMetrics,
}

/// Metrics collector that aggregates data from the trading system
pub struct MetricsCollector {
    portfolio_metrics: Arc<RwLock<PortfolioMetrics>>,
    signal_metrics: Arc<RwLock<SignalMetrics>>,
    position_metrics: Arc<RwLock<Vec<PositionMetrics>>>,
    market_metrics: Arc<RwLock<HashMap<Symbol, MarketMetrics>>>,
    risk_metrics: Arc<RwLock<RiskMetrics>>,
    
    // Signal processing counters
    signal_count: Arc<RwLock<u64>>,
    signal_history: Arc<RwLock<Vec<TradingSignal>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let now = Utc::now();
        
        Self {
            portfolio_metrics: Arc::new(RwLock::new(PortfolioMetrics {
                timestamp: now,
                total_capital: 0.0,
                available_capital: 0.0,
                total_pnl: 0.0,
                unrealized_pnl: 0.0,
                realized_pnl: 0.0,
                total_return_pct: 0.0,
                positions_count: 0,
                active_positions_count: 0,
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                win_rate: 0.0,
                avg_win: 0.0,
                avg_loss: 0.0,
                max_drawdown: 0.0,
                sharpe_ratio: 0.0,
            })),
            signal_metrics: Arc::new(RwLock::new(SignalMetrics {
                timestamp: now,
                signals_processed: 0,
                signals_per_minute: 0.0,
                avg_confidence: 0.0,
                avg_urgency: 0.0,
                signal_distribution: HashMap::new(),
                pattern_strength_avg: 0.0,
                spike_count_avg: 0.0,
                volatility_avg: 0.0,
                market_regimes: HashMap::new(),
            })),
            position_metrics: Arc::new(RwLock::new(Vec::new())),
            market_metrics: Arc::new(RwLock::new(HashMap::new())),
            risk_metrics: Arc::new(RwLock::new(RiskMetrics {
                timestamp: now,
                portfolio_var_95: 0.0,
                portfolio_var_99: 0.0,
                max_position_size_pct: 0.0,
                current_leverage: 0.0,
                correlation_btc: 0.0,
                correlation_eth: 0.0,
                concentration_risk: 0.0,
                daily_volatility: 0.0,
            })),
            signal_count: Arc::new(RwLock::new(0)),
            signal_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Update portfolio metrics from trading statistics
    pub fn update_portfolio_metrics(&self, stats: &crate::paper_trading::TradingStatistics) {
        let mut metrics = self.portfolio_metrics.write();
        metrics.timestamp = Utc::now();
        metrics.total_capital = stats.capital;
        metrics.total_pnl = stats.total_pnl;
        metrics.total_return_pct = stats.total_return_pct;
        metrics.positions_count = stats.position_stats.total_positions as usize;
        metrics.active_positions_count = stats.position_stats.open_positions as usize;
        metrics.winning_trades = stats.position_stats.winning_positions as usize;
        metrics.losing_trades = stats.position_stats.losing_positions as usize;
        metrics.total_trades = (stats.position_stats.winning_positions + stats.position_stats.losing_positions) as usize;
        
        if metrics.total_trades > 0 {
            metrics.win_rate = metrics.winning_trades as f64 / metrics.total_trades as f64;
        }
        
        metrics.avg_win = stats.position_stats.avg_win;
        metrics.avg_loss = stats.position_stats.avg_loss;
        metrics.max_drawdown = 0.0; // TODO: Calculate from returns history
        
        // Calculate Sharpe ratio if we have risk metrics
        metrics.sharpe_ratio = stats.risk_metrics.sharpe_ratio;
    }

    /// Record a new trading signal
    pub fn record_signal(&self, signal: &TradingSignal) {
        {
            let mut count = self.signal_count.write();
            *count += 1;
        }

        {
            let mut history = self.signal_history.write();
            history.push(signal.clone());
            
            // Keep only last 1000 signals
            if history.len() > 1000 {
                history.remove(0);
            }
        }

        self.update_signal_metrics();
    }

    /// Update signal metrics based on recent signal history
    fn update_signal_metrics(&self) {
        let history = self.signal_history.read();
        let count = self.signal_count.read();
        let mut metrics = self.signal_metrics.write();

        metrics.timestamp = Utc::now();
        metrics.signals_processed = *count;

        if !history.is_empty() {
            // Calculate averages
            let total_confidence: f64 = history.iter().map(|s| s.confidence).sum();
            let total_urgency: f64 = history.iter().map(|s| s.urgency).sum();
            let total_pattern_strength: f64 = history.iter().map(|s| s.metadata.pattern_strength).sum();
            let total_spike_count: f64 = history.iter().map(|s| s.metadata.spike_count as f64).sum();
            let total_volatility: f64 = history.iter().map(|s| s.metadata.volatility).sum();

            let len = history.len() as f64;
            metrics.avg_confidence = total_confidence / len;
            metrics.avg_urgency = total_urgency / len;
            metrics.pattern_strength_avg = total_pattern_strength / len;
            metrics.spike_count_avg = total_spike_count / len;
            metrics.volatility_avg = total_volatility / len;

            // Count signal types
            let mut distribution = HashMap::new();
            let mut regimes = HashMap::new();

            for signal in history.iter() {
                let action_type = match &signal.action {
                    crate::paper_trading::SignalAction::Buy { .. } => "Buy",
                    crate::paper_trading::SignalAction::Sell { .. } => "Sell",
                    crate::paper_trading::SignalAction::Hold => "Hold",
                    crate::paper_trading::SignalAction::Close { .. } => "Close",
                };
                *distribution.entry(action_type.to_string()).or_insert(0) += 1;
                *regimes.entry(signal.metadata.market_regime.clone()).or_insert(0) += 1;
            }

            metrics.signal_distribution = distribution;
            metrics.market_regimes = regimes;

            // Calculate signals per minute (last 10 minutes)
            let ten_minutes_ago = Utc::now() - chrono::Duration::minutes(10);
            let recent_signals = history.iter()
                .filter(|_| true) // TODO: Add timestamp to TradingSignal
                .count();
            metrics.signals_per_minute = recent_signals as f64 / 10.0;
        }
    }

    /// Update market data metrics
    pub fn update_market_data(&self, symbol: Symbol, price: f64) {
        let mut market_data = self.market_metrics.write();
        
        // Calculate price change if we have previous data
        let (price_change_24h, price_change_pct_24h) = if let Some(existing) = market_data.get(&symbol) {
            let change = price - existing.price;
            let change_pct = if existing.price > 0.0 {
                (change / existing.price) * 100.0
            } else {
                0.0
            };
            (change, change_pct)
        } else {
            (0.0, 0.0)
        };
        
        let metric = MarketMetrics {
            timestamp: Utc::now(),
            symbol: symbol.to_string(),
            price,
            volume_24h: 1_500_000.0, // Simulated volume for demo
            price_change_24h,
            price_change_pct_24h,
            volatility: (price_change_pct_24h.abs() * 0.5).min(5.0), // Estimate volatility from price change
            last_update: Utc::now(),
        };
        
        market_data.insert(symbol, metric);
    }

    /// Get all current metrics for Grafana
    pub fn get_all_metrics(&self) -> TradingMetrics {
        TradingMetrics {
            portfolio: self.portfolio_metrics.read().clone(),
            signals: self.signal_metrics.read().clone(),
            positions: self.position_metrics.read().clone(),
            market_data: self.market_metrics.read().values().cloned().collect(),
            risk: self.risk_metrics.read().clone(),
        }
    }

    /// Get portfolio metrics only
    pub fn get_portfolio_metrics(&self) -> PortfolioMetrics {
        self.portfolio_metrics.read().clone()
    }

    /// Get signal metrics only  
    pub fn get_signal_metrics(&self) -> SignalMetrics {
        self.signal_metrics.read().clone()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}