use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{MarketData, TradingOpportunity};
use crate::exchanges::{Symbol, Side};
use chrono::Utc;
use async_trait::async_trait;

#[derive(Clone)]
pub struct StrategyEngine {
    strategies: Vec<Box<dyn TradingStrategy>>,
    market_history: HashMap<String, Vec<MarketData>>,
    max_history_length: usize,
}

#[async_trait]
pub trait TradingStrategy: Send + Sync {
    async fn analyze(&self, data: &MarketData, history: &[MarketData]) -> Result<Vec<TradingOpportunity>>;
    fn get_name(&self) -> &str;
    fn get_description(&self) -> &str;
    fn get_risk_level(&self) -> RiskLevel;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Conservative,
    Moderate,
    Aggressive,
    HighRisk,
}

#[derive(Clone)]
pub struct MomentumBreakoutStrategy {
    min_volume_spike: f64,
    min_price_change: f64,
    consolidation_periods: usize,
}

#[derive(Clone)]
pub struct VolumeSpikeMomentumStrategy {
    volume_threshold: f64,
    price_threshold: f64,
    confirmation_periods: usize,
}

#[derive(Clone)]
pub struct GapAndGoStrategy {
    min_gap_percent: f64,
    min_volume_ratio: f64,
    max_pullback_percent: f64,
}

#[derive(Clone)]
pub struct RelativeStrengthStrategy {
    lookback_periods: usize,
    min_rs_threshold: f64,
    market_correlation_threshold: f64,
}

#[derive(Clone)]
pub struct VolatilityBreakoutStrategy {
    atr_multiplier: f64,
    consolidation_threshold: f64,
    breakout_volume_threshold: f64,
}

#[derive(Clone)]
pub struct NeuromorphicMomentumStrategy {
    neural_confidence_threshold: f64,
    pattern_strength_threshold: f64,
    spike_density_threshold: f64,
}

impl StrategyEngine {
    pub fn new() -> Self {
        let mut strategies: Vec<Box<dyn TradingStrategy>> = Vec::new();
        
        strategies.push(Box::new(MomentumBreakoutStrategy::new()));
        strategies.push(Box::new(VolumeSpikeMomentumStrategy::new()));
        strategies.push(Box::new(GapAndGoStrategy::new()));
        strategies.push(Box::new(RelativeStrengthStrategy::new()));
        strategies.push(Box::new(VolatilityBreakoutStrategy::new()));
        strategies.push(Box::new(NeuromorphicMomentumStrategy::new()));

        Self {
            strategies,
            market_history: HashMap::new(),
            max_history_length: 100,
        }
    }

    pub async fn analyze_opportunity(&self, data: &MarketData) -> Result<Vec<TradingOpportunity>> {
        self.update_history(data).await;
        
        let history = self.market_history
            .get(&data.symbol.name)
            .map(|h| h.as_slice())
            .unwrap_or(&[]);

        let mut all_opportunities = Vec::new();
        
        for strategy in &self.strategies {
            if let Ok(opportunities) = strategy.analyze(data, history).await {
                all_opportunities.extend(opportunities);
            }
        }

        all_opportunities.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        Ok(all_opportunities)
    }

    async fn update_history(&self, data: &MarketData) {
        // For now, just store the latest data point
        // In a full implementation, this would maintain proper history
    }
}

impl MomentumBreakoutStrategy {
    pub fn new() -> Self {
        Self {
            min_volume_spike: 2.0,
            min_price_change: 3.0,
            consolidation_periods: 10,
        }
    }
}

#[async_trait]
impl TradingStrategy for MomentumBreakoutStrategy {
    async fn analyze(&self, data: &MarketData, history: &[MarketData]) -> Result<Vec<TradingOpportunity>> {
        let mut opportunities = Vec::new();

        if history.len() < self.consolidation_periods {
            return Ok(opportunities);
        }

        let avg_volume = history.iter()
            .map(|h| h.volume)
            .sum::<f64>() / history.len() as f64;
        
        let volume_spike = data.volume / avg_volume;
        let price_change = data.change_24h.abs();

        if volume_spike > self.min_volume_spike && price_change > self.min_price_change {
            let is_consolidating = self.is_consolidating(history);
            
            if is_consolidating {
                let side = if data.change_24h > 0.0 { Side::Buy } else { Side::Sell };
                let confidence = ((volume_spike - self.min_volume_spike) * 0.2 + 
                                (price_change - self.min_price_change) * 0.1).min(0.95);

                opportunities.push(TradingOpportunity {
                    symbol: data.symbol.clone(),
                    strategy: "Momentum Breakout".to_string(),
                    confidence,
                    expected_move: price_change * 1.5,
                    time_horizon: "1-3 days".to_string(),
                    entry_price: data.price,
                    stop_loss: Some(data.price * if side == Side::Buy { 0.95 } else { 1.05 }),
                    take_profit: Some(data.price * if side == Side::Buy { 1.08 } else { 0.92 }),
                    position_size: self.calculate_position_size(confidence),
                    reasoning: format!(
                        "Volume spike {:.1}x with {:.1}% price move after consolidation",
                        volume_spike, price_change
                    ),
                    risk_score: 1.0 - confidence,
                    timestamp: Utc::now(),
                });
            }
        }

        Ok(opportunities)
    }

    fn get_name(&self) -> &str {
        "Momentum Breakout"
    }

    fn get_description(&self) -> &str {
        "Identifies breakouts from consolidation periods with volume confirmation"
    }

    fn get_risk_level(&self) -> RiskLevel {
        RiskLevel::Moderate
    }
}

impl MomentumBreakoutStrategy {
    fn is_consolidating(&self, history: &[MarketData]) -> bool {
        if history.len() < self.consolidation_periods {
            return false;
        }

        let recent_data = &history[history.len() - self.consolidation_periods..];
        let price_range = recent_data.iter()
            .map(|d| d.high - d.low)
            .sum::<f64>() / recent_data.len() as f64;
        
        let avg_price = recent_data.iter()
            .map(|d| d.price)
            .sum::<f64>() / recent_data.len() as f64;

        (price_range / avg_price) < 0.05
    }

    fn calculate_position_size(&self, confidence: f64) -> f64 {
        (confidence * 0.1).min(0.05)
    }
}

impl VolumeSpikeMomentumStrategy {
    pub fn new() -> Self {
        Self {
            volume_threshold: 3.0,
            price_threshold: 2.0,
            confirmation_periods: 3,
        }
    }
}

#[async_trait]
impl TradingStrategy for VolumeSpikeMomentumStrategy {
    async fn analyze(&self, data: &MarketData, history: &[MarketData]) -> Result<Vec<TradingOpportunity>> {
        let mut opportunities = Vec::new();

        if history.is_empty() {
            return Ok(opportunities);
        }

        let avg_volume = history.iter()
            .map(|h| h.volume)
            .sum::<f64>() / history.len() as f64;

        let volume_ratio = data.volume / avg_volume;
        let price_change = data.change_24h.abs();

        if volume_ratio > self.volume_threshold && price_change > self.price_threshold {
            let momentum_confirmed = self.confirm_momentum(data, history);
            
            if momentum_confirmed {
                let side = if data.change_24h > 0.0 { Side::Buy } else { Side::Sell };
                let confidence = (volume_ratio * 0.15 + price_change * 0.1).min(0.9);

                opportunities.push(TradingOpportunity {
                    symbol: data.symbol.clone(),
                    strategy: "Volume Spike Momentum".to_string(),
                    confidence,
                    expected_move: price_change * 2.0,
                    time_horizon: "4-8 hours".to_string(),
                    entry_price: data.price,
                    stop_loss: Some(data.price * if side == Side::Buy { 0.97 } else { 1.03 }),
                    take_profit: Some(data.price * if side == Side::Buy { 1.06 } else { 0.94 }),
                    position_size: (confidence * 0.08).min(0.04),
                    reasoning: format!(
                        "Volume spike {:.1}x normal with {:.1}% momentum",
                        volume_ratio, price_change
                    ),
                    risk_score: 1.0 - confidence + 0.1,
                    timestamp: Utc::now(),
                });
            }
        }

        Ok(opportunities)
    }

    fn get_name(&self) -> &str {
        "Volume Spike Momentum"
    }

    fn get_description(&self) -> &str {
        "Captures momentum plays based on unusual volume spikes"
    }

    fn get_risk_level(&self) -> RiskLevel {
        RiskLevel::Aggressive
    }
}

impl VolumeSpikeMomentumStrategy {
    fn confirm_momentum(&self, data: &MarketData, history: &[MarketData]) -> bool {
        if history.len() < self.confirmation_periods {
            return false;
        }

        let recent = &history[history.len() - self.confirmation_periods..];
        let trend_direction = if data.change_24h > 0.0 { 1.0 } else { -1.0 };
        
        let momentum_score = recent.iter()
            .map(|h| if h.change_24h * trend_direction > 0.0 { 1.0 } else { 0.0 })
            .sum::<f64>() / recent.len() as f64;

        momentum_score > 0.6
    }
}

impl NeuromorphicMomentumStrategy {
    pub fn new() -> Self {
        Self {
            neural_confidence_threshold: 0.75,
            pattern_strength_threshold: 0.8,
            spike_density_threshold: 0.7,
        }
    }
}

#[async_trait]
impl TradingStrategy for NeuromorphicMomentumStrategy {
    async fn analyze(&self, data: &MarketData, history: &[MarketData]) -> Result<Vec<TradingOpportunity>> {
        let mut opportunities = Vec::new();

        let neural_signal = self.calculate_neuromorphic_signal(data, history).await?;
        
        if neural_signal.confidence > self.neural_confidence_threshold {
            let side = if neural_signal.direction > 0.0 { Side::Buy } else { Side::Sell };
            
            opportunities.push(TradingOpportunity {
                symbol: data.symbol.clone(),
                strategy: "Neuromorphic AI Momentum".to_string(),
                confidence: neural_signal.confidence,
                expected_move: neural_signal.expected_move,
                time_horizon: "2-6 hours".to_string(),
                entry_price: data.price,
                stop_loss: Some(data.price * if side == Side::Buy { 0.96 } else { 1.04 }),
                take_profit: Some(data.price * if side == Side::Buy { 1.12 } else { 0.88 }),
                position_size: (neural_signal.confidence * 0.12).min(0.06),
                reasoning: format!(
                    "Neural pattern recognition: {:.0}% confidence, {:.1}% expected move",
                    neural_signal.confidence * 100.0, neural_signal.expected_move
                ),
                risk_score: 1.0 - neural_signal.confidence,
                timestamp: Utc::now(),
            });
        }

        Ok(opportunities)
    }

    fn get_name(&self) -> &str {
        "Neuromorphic AI Momentum"
    }

    fn get_description(&self) -> &str {
        "Advanced AI pattern recognition for momentum detection"
    }

    fn get_risk_level(&self) -> RiskLevel {
        RiskLevel::HighRisk
    }
}

#[derive(Debug)]
struct NeuralSignal {
    confidence: f64,
    direction: f64,
    expected_move: f64,
    pattern_strength: f64,
}

impl NeuromorphicMomentumStrategy {
    async fn calculate_neuromorphic_signal(&self, data: &MarketData, history: &[MarketData]) -> Result<NeuralSignal> {
        let pattern_strength = self.analyze_price_patterns(data, history);
        let volume_pattern = self.analyze_volume_patterns(data, history);
        let momentum_persistence = self.calculate_momentum_persistence(history);
        
        let spike_density = self.calculate_spike_density(data, history);
        let volatility_signature = self.analyze_volatility_signature(data, history);
        
        let confidence = (pattern_strength * 0.3 + 
                         volume_pattern * 0.25 + 
                         momentum_persistence * 0.2 + 
                         spike_density * 0.15 + 
                         volatility_signature * 0.1).min(0.95);
        
        let direction = if data.change_24h > 0.0 { 1.0 } else { -1.0 };
        let expected_move = data.change_24h.abs() * (1.0 + confidence);

        Ok(NeuralSignal {
            confidence,
            direction,
            expected_move,
            pattern_strength,
        })
    }

    fn analyze_price_patterns(&self, data: &MarketData, history: &[MarketData]) -> f64 {
        if history.len() < 10 {
            return 0.5;
        }

        let price_changes: Vec<f64> = history.windows(2)
            .map(|w| (w[1].price - w[0].price) / w[0].price)
            .collect();

        let acceleration = if price_changes.len() > 3 {
            let recent_avg = price_changes[price_changes.len()-3..].iter().sum::<f64>() / 3.0;
            let earlier_avg = price_changes[price_changes.len()-6..price_changes.len()-3].iter().sum::<f64>() / 3.0;
            (recent_avg - earlier_avg).abs()
        } else {
            0.0
        };

        (acceleration * 50.0).min(1.0)
    }

    fn analyze_volume_patterns(&self, data: &MarketData, history: &[MarketData]) -> f64 {
        if history.is_empty() {
            return 0.5;
        }

        let avg_volume = history.iter().map(|h| h.volume).sum::<f64>() / history.len() as f64;
        let volume_ratio = data.volume / avg_volume;
        
        (volume_ratio / 5.0).min(1.0)
    }

    fn calculate_momentum_persistence(&self, history: &[MarketData]) -> f64 {
        if history.len() < 5 {
            return 0.5;
        }

        let direction_consistency = history.windows(2)
            .map(|w| if (w[1].price - w[0].price) * (w[1].change_24h) > 0.0 { 1.0 } else { 0.0 })
            .sum::<f64>() / (history.len() - 1) as f64;

        direction_consistency
    }

    fn calculate_spike_density(&self, data: &MarketData, history: &[MarketData]) -> f64 {
        let price_volatility = (data.high - data.low) / data.price;
        let volume_spike = if !history.is_empty() {
            let avg_vol = history.iter().map(|h| h.volume).sum::<f64>() / history.len() as f64;
            data.volume / avg_vol
        } else {
            1.0
        };

        ((price_volatility * 10.0 + volume_spike / 5.0) / 2.0).min(1.0)
    }

    fn analyze_volatility_signature(&self, data: &MarketData, history: &[MarketData]) -> f64 {
        if history.len() < 5 {
            return 0.5;
        }

        let recent_volatility = history[history.len()-5..]
            .iter()
            .map(|h| (h.high - h.low) / h.price)
            .sum::<f64>() / 5.0;

        let current_volatility = (data.high - data.low) / data.price;
        
        if current_volatility > recent_volatility * 1.5 {
            0.8
        } else if current_volatility > recent_volatility {
            0.6
        } else {
            0.4
        }
    }
}

impl GapAndGoStrategy {
    pub fn new() -> Self {
        Self {
            min_gap_percent: 2.0,
            min_volume_ratio: 1.5,
            max_pullback_percent: 30.0,
        }
    }
}

#[async_trait]
impl TradingStrategy for GapAndGoStrategy {
    async fn analyze(&self, data: &MarketData, _history: &[MarketData]) -> Result<Vec<TradingOpportunity>> {
        let mut opportunities = Vec::new();
        
        let gap_percent = ((data.open - data.price) / data.price * 100.0).abs();
        
        if gap_percent > self.min_gap_percent {
            let side = if data.open > data.price { Side::Buy } else { Side::Sell };
            let confidence = (gap_percent / 10.0).min(0.85);
            
            opportunities.push(TradingOpportunity {
                symbol: data.symbol.clone(),
                strategy: "Gap and Go".to_string(),
                confidence,
                expected_move: gap_percent * 0.8,
                time_horizon: "1-2 hours".to_string(),
                entry_price: data.price,
                stop_loss: Some(data.price * if side == Side::Buy { 0.98 } else { 1.02 }),
                take_profit: Some(data.price * if side == Side::Buy { 1.04 } else { 0.96 }),
                position_size: (confidence * 0.06).min(0.03),
                reasoning: format!("Gap {:.1}% with continuation potential", gap_percent),
                risk_score: 1.0 - confidence + 0.2,
                timestamp: Utc::now(),
            });
        }
        
        Ok(opportunities)
    }

    fn get_name(&self) -> &str {
        "Gap and Go"
    }

    fn get_description(&self) -> &str {
        "Identifies gap up/down opportunities with continuation potential"
    }

    fn get_risk_level(&self) -> RiskLevel {
        RiskLevel::Aggressive
    }
}

impl RelativeStrengthStrategy {
    pub fn new() -> Self {
        Self {
            lookback_periods: 20,
            min_rs_threshold: 0.7,
            market_correlation_threshold: 0.3,
        }
    }
}

#[async_trait]
impl TradingStrategy for RelativeStrengthStrategy {
    async fn analyze(&self, data: &MarketData, history: &[MarketData]) -> Result<Vec<TradingOpportunity>> {
        let opportunities = Vec::new();
        
        Ok(opportunities)
    }

    fn get_name(&self) -> &str {
        "Relative Strength"
    }

    fn get_description(&self) -> &str {
        "Identifies stocks with strong relative strength vs market"
    }

    fn get_risk_level(&self) -> RiskLevel {
        RiskLevel::Moderate
    }
}

impl VolatilityBreakoutStrategy {
    pub fn new() -> Self {
        Self {
            atr_multiplier: 2.0,
            consolidation_threshold: 0.02,
            breakout_volume_threshold: 1.8,
        }
    }
}

#[async_trait]
impl TradingStrategy for VolatilityBreakoutStrategy {
    async fn analyze(&self, data: &MarketData, history: &[MarketData]) -> Result<Vec<TradingOpportunity>> {
        let opportunities = Vec::new();
        
        Ok(opportunities)
    }

    fn get_name(&self) -> &str {
        "Volatility Breakout"
    }

    fn get_description(&self) -> &str {
        "Captures breakouts from low volatility consolidations"
    }

    fn get_risk_level(&self) -> RiskLevel {
        RiskLevel::Moderate
    }
}