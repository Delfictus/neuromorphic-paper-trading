//! Risk management for paper trading

use crate::exchanges::{Symbol, Side};
use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Risk limits configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskLimits {
    pub max_position_size: f64,
    pub max_daily_loss: f64,
    pub max_drawdown: f64,
    pub max_leverage: f64,
    pub max_positions: usize,
    pub max_orders_per_minute: u64,
    pub max_correlation: f64,
    pub position_size_pct: f64,  // % of capital per position
    pub stop_loss_pct: f64,      // Default stop loss %
    pub take_profit_pct: f64,    // Default take profit %
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_position_size: 100000.0,
            max_daily_loss: 5000.0,
            max_drawdown: 0.2,  // 20%
            max_leverage: 3.0,
            max_positions: 10,
            max_orders_per_minute: 100,
            max_correlation: 0.7,
            position_size_pct: 2.0,  // 2% per position
            stop_loss_pct: 2.0,      // 2% stop loss
            take_profit_pct: 4.0,    // 4% take profit
        }
    }
}

/// Risk metrics
#[derive(Default, Clone, Debug)]
pub struct RiskMetrics {
    pub current_drawdown: f64,
    pub max_drawdown: f64,
    pub daily_pnl: f64,
    pub total_exposure: f64,
    pub leverage_ratio: f64,
    pub var_95: f64,  // Value at Risk 95%
    pub var_99: f64,  // Value at Risk 99%
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
}

/// Risk check result
#[derive(Clone, Debug)]
pub enum RiskCheckResult {
    Approved,
    Rejected { reason: String },
    Warning { message: String },
}

/// Kelly Criterion calculator
pub struct KellyCriterion {
    win_rate: f64,
    avg_win: f64,
    avg_loss: f64,
}

impl KellyCriterion {
    pub fn new(win_rate: f64, avg_win: f64, avg_loss: f64) -> Self {
        Self {
            win_rate,
            avg_win,
            avg_loss,
        }
    }
    
    /// Calculate optimal position size using Kelly Criterion
    pub fn calculate_position_size(&self, capital: f64, max_fraction: f64) -> f64 {
        if self.avg_loss == 0.0 {
            return 0.0;
        }
        
        let b = self.avg_win / self.avg_loss;
        let p = self.win_rate;
        let q = 1.0 - p;
        
        // Kelly fraction: f = (bp - q) / b
        let kelly_fraction = (b * p - q) / b;
        
        // Apply Kelly fraction cap (usually 25% of full Kelly)
        let safe_fraction = kelly_fraction.min(max_fraction).max(0.0);
        
        capital * safe_fraction
    }
}

/// Portfolio heat map for correlation tracking
pub struct PortfolioHeatMap {
    correlations: DashMap<(Symbol, Symbol), f64>,
    returns_history: DashMap<Symbol, Vec<f64>>,
    window_size: usize,
}

impl PortfolioHeatMap {
    pub fn new(window_size: usize) -> Self {
        Self {
            correlations: DashMap::new(),
            returns_history: DashMap::new(),
            window_size,
        }
    }
    
    /// Update returns for a symbol
    pub fn update_returns(&self, symbol: Symbol, return_pct: f64) {
        let mut history = self.returns_history
            .entry(symbol)
            .or_insert_with(Vec::new);
        
        history.push(return_pct);
        if history.len() > self.window_size {
            history.remove(0);
        }
    }
    
    /// Calculate correlation between two symbols
    pub fn calculate_correlation(&self, symbol1: &Symbol, symbol2: &Symbol) -> Option<f64> {
        let history1 = self.returns_history.get(symbol1)?;
        let history2 = self.returns_history.get(symbol2)?;
        
        if history1.len() < 20 || history2.len() < 20 {
            return None;
        }
        
        let n = history1.len().min(history2.len()) as f64;
        let mean1 = history1.iter().sum::<f64>() / n;
        let mean2 = history2.iter().sum::<f64>() / n;
        
        let mut covariance = 0.0;
        let mut var1 = 0.0;
        let mut var2 = 0.0;
        
        for i in 0..n as usize {
            let diff1 = history1[i] - mean1;
            let diff2 = history2[i] - mean2;
            covariance += diff1 * diff2;
            var1 += diff1 * diff1;
            var2 += diff2 * diff2;
        }
        
        if var1 == 0.0 || var2 == 0.0 {
            return None;
        }
        
        let correlation = covariance / (var1.sqrt() * var2.sqrt());
        self.correlations.insert((symbol1.clone(), symbol2.clone()), correlation);
        
        Some(correlation)
    }
    
    /// Get portfolio concentration risk
    pub fn get_concentration_risk(&self, positions: &[(Symbol, f64)]) -> f64 {
        if positions.is_empty() {
            return 0.0;
        }
        
        let total_value: f64 = positions.iter().map(|(_, v)| v.abs()).sum();
        let mut max_concentration = 0.0;
        
        for (_, value) in positions {
            let concentration = value.abs() / total_value;
            max_concentration = max_concentration.max(concentration);
        }
        
        max_concentration
    }
}

/// Risk manager
pub struct RiskManager {
    limits: RiskLimits,
    metrics: Arc<parking_lot::RwLock<RiskMetrics>>,
    portfolio_heat_map: Arc<PortfolioHeatMap>,
    kelly_criterion: Arc<parking_lot::RwLock<KellyCriterion>>,
    daily_loss: Arc<parking_lot::RwLock<f64>>,
    peak_capital: Arc<parking_lot::RwLock<f64>>,
    orders_per_minute: Arc<AtomicU64>,
    position_count: Arc<AtomicU64>,
}

impl RiskManager {
    pub fn new(limits: RiskLimits, initial_capital: f64) -> Self {
        Self {
            limits,
            metrics: Arc::new(parking_lot::RwLock::new(RiskMetrics::default())),
            portfolio_heat_map: Arc::new(PortfolioHeatMap::new(100)),
            kelly_criterion: Arc::new(parking_lot::RwLock::new(KellyCriterion::new(0.5, 2.0, 1.0))),
            daily_loss: Arc::new(parking_lot::RwLock::new(0.0)),
            peak_capital: Arc::new(parking_lot::RwLock::new(initial_capital)),
            orders_per_minute: Arc::new(AtomicU64::new(0)),
            position_count: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Check if order should be allowed
    pub fn check_order(
        &self,
        symbol: &Symbol,
        side: Side,
        quantity: f64,
        price: f64,
        current_capital: f64,
    ) -> RiskCheckResult {
        // Check order rate limit
        let orders_count = self.orders_per_minute.load(Ordering::Relaxed);
        if orders_count >= self.limits.max_orders_per_minute {
            return RiskCheckResult::Rejected {
                reason: format!(
                    "Order rate limit exceeded: {} orders/min",
                    orders_count
                )
            };
        }
        
        // Check position count
        let pos_count = self.position_count.load(Ordering::Relaxed) as usize;
        if pos_count >= self.limits.max_positions {
            return RiskCheckResult::Rejected {
                reason: format!(
                    "Max positions limit reached: {}/{}",
                    pos_count, self.limits.max_positions
                )
            };
        }
        
        // Check position size
        let position_value = quantity * price;
        if position_value > self.limits.max_position_size {
            return RiskCheckResult::Rejected {
                reason: format!(
                    "Position size ${:.2} exceeds limit ${:.2}",
                    position_value, self.limits.max_position_size
                )
            };
        }
        
        // Check daily loss limit
        let daily_loss = *self.daily_loss.read();
        if daily_loss.abs() > self.limits.max_daily_loss {
            return RiskCheckResult::Rejected {
                reason: format!(
                    "Daily loss limit exceeded: ${:.2}/${:.2}",
                    daily_loss.abs(), self.limits.max_daily_loss
                )
            };
        }
        
        // Check leverage
        let metrics = self.metrics.read();
        let new_exposure = metrics.total_exposure + position_value;
        let leverage = new_exposure / current_capital;
        
        if leverage > self.limits.max_leverage {
            return RiskCheckResult::Rejected {
                reason: format!(
                    "Leverage limit exceeded: {:.2}x/{:.2}x",
                    leverage, self.limits.max_leverage
                )
            };
        }
        
        // Check drawdown
        if metrics.current_drawdown > self.limits.max_drawdown {
            return RiskCheckResult::Warning {
                message: format!(
                    "High drawdown: {:.1}%",
                    metrics.current_drawdown * 100.0
                )
            };
        }
        
        RiskCheckResult::Approved
    }
    
    /// Calculate optimal position size
    pub fn calculate_position_size(
        &self,
        symbol: &Symbol,
        current_capital: f64,
        confidence: f64,
    ) -> f64 {
        // Use Kelly Criterion for sizing
        let kelly = self.kelly_criterion.read();
        let kelly_size = kelly.calculate_position_size(current_capital, 0.25); // 25% of full Kelly
        
        // Apply position size percentage limit
        let pct_size = current_capital * (self.limits.position_size_pct / 100.0);
        
        // Apply confidence adjustment
        let confidence_adjusted = pct_size * confidence.min(1.0).max(0.1);
        
        // Return minimum of all constraints
        kelly_size.min(pct_size).min(confidence_adjusted).min(self.limits.max_position_size)
    }
    
    /// Update risk metrics
    pub fn update_metrics(
        &self,
        current_capital: f64,
        total_exposure: f64,
        daily_pnl: f64,
        returns: &[f64],
    ) {
        let mut metrics = self.metrics.write();
        let mut peak = self.peak_capital.write();
        
        // Update peak capital
        if current_capital > *peak {
            *peak = current_capital;
        }
        
        // Calculate drawdown
        metrics.current_drawdown = (*peak - current_capital) / *peak;
        metrics.max_drawdown = metrics.max_drawdown.max(metrics.current_drawdown);
        
        // Update exposure and leverage
        metrics.total_exposure = total_exposure;
        metrics.leverage_ratio = if current_capital > 0.0 {
            total_exposure / current_capital
        } else {
            0.0
        };
        
        // Update daily P&L
        metrics.daily_pnl = daily_pnl;
        *self.daily_loss.write() = daily_pnl.min(0.0);
        
        // Calculate VaR if we have enough data
        if returns.len() > 20 {
            let mut sorted_returns = returns.to_vec();
            sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            let var_95_idx = (returns.len() as f64 * 0.05) as usize;
            let var_99_idx = (returns.len() as f64 * 0.01) as usize;
            
            metrics.var_95 = sorted_returns[var_95_idx].abs() * current_capital;
            metrics.var_99 = sorted_returns[var_99_idx].abs() * current_capital;
        }
        
        // Calculate Sharpe ratio
        if returns.len() > 1 {
            let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance = returns.iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>() / returns.len() as f64;
            let std_dev = variance.sqrt();
            
            if std_dev > 0.0 {
                metrics.sharpe_ratio = (mean_return * 252.0_f64.sqrt()) / std_dev; // Annualized
            }
        }
        
        // Calculate Sortino ratio (downside deviation)
        if returns.len() > 1 {
            let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
            let downside_returns: Vec<f64> = returns.iter()
                .filter(|&&r| r < 0.0)
                .copied()
                .collect();
            
            if !downside_returns.is_empty() {
                let downside_variance = downside_returns.iter()
                    .map(|r| r.powi(2))
                    .sum::<f64>() / downside_returns.len() as f64;
                let downside_dev = downside_variance.sqrt();
                
                if downside_dev > 0.0 {
                    metrics.sortino_ratio = (mean_return * 252.0_f64.sqrt()) / downside_dev;
                }
            }
        }
        
        // Calculate Calmar ratio
        if metrics.max_drawdown > 0.0 && returns.len() > 252 {
            let annual_return = returns.iter().sum::<f64>();
            metrics.calmar_ratio = annual_return / metrics.max_drawdown;
        }
    }
    
    /// Update Kelly Criterion parameters
    pub fn update_kelly_parameters(&self, win_rate: f64, avg_win: f64, avg_loss: f64) {
        *self.kelly_criterion.write() = KellyCriterion::new(win_rate, avg_win, avg_loss);
    }
    
    /// Record order for rate limiting
    pub fn record_order(&self) {
        self.orders_per_minute.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Reset daily metrics
    pub fn reset_daily_metrics(&self) {
        *self.daily_loss.write() = 0.0;
        self.orders_per_minute.store(0, Ordering::Relaxed);
        
        let mut metrics = self.metrics.write();
        metrics.daily_pnl = 0.0;
    }
    
    /// Get current risk metrics
    pub fn get_metrics(&self) -> RiskMetrics {
        self.metrics.read().clone()
    }
    
    /// Get risk limits
    pub fn get_limits(&self) -> &RiskLimits {
        &self.limits
    }
    
    /// Check portfolio correlation risk
    pub fn check_correlation_risk(&self, positions: &[(Symbol, f64)]) -> RiskCheckResult {
        let concentration = self.portfolio_heat_map.get_concentration_risk(positions);
        
        if concentration > 0.3 {
            return RiskCheckResult::Warning {
                message: format!("High concentration risk: {:.1}%", concentration * 100.0)
            };
        }
        
        // Check pairwise correlations
        for i in 0..positions.len() {
            for j in i+1..positions.len() {
                if let Some(corr) = self.portfolio_heat_map.calculate_correlation(
                    &positions[i].0,
                    &positions[j].0
                ) {
                    if corr.abs() > self.limits.max_correlation {
                        return RiskCheckResult::Warning {
                            message: format!(
                                "High correlation between {} and {}: {:.2}",
                                positions[i].0, positions[j].0, corr
                            )
                        };
                    }
                }
            }
        }
        
        RiskCheckResult::Approved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_kelly_criterion() {
        let kelly = KellyCriterion::new(0.6, 2.0, 1.0);
        let position_size = kelly.calculate_position_size(10000.0, 0.25);
        
        // Kelly fraction = (2*0.6 - 0.4)/2 = 0.4
        // With 25% cap = 0.25
        // Position = 10000 * 0.25 = 2500
        assert_eq!(position_size, 2500.0);
    }
    
    #[test]
    fn test_risk_checks() {
        let limits = RiskLimits::default();
        let manager = RiskManager::new(limits, 100000.0);
        
        // Should approve reasonable order
        let result = manager.check_order(
            &Symbol::new("BTC-USD"),
            Side::Buy,
            1.0,
            50000.0,
            100000.0
        );
        
        match result {
            RiskCheckResult::Approved => {},
            _ => panic!("Expected approval"),
        }
        
        // Should reject overleveraged order
        let result = manager.check_order(
            &Symbol::new("BTC-USD"),
            Side::Buy,
            10.0,
            50000.0,
            100000.0
        );
        
        match result {
            RiskCheckResult::Rejected { .. } => {},
            _ => panic!("Expected rejection"),
        }
    }
}