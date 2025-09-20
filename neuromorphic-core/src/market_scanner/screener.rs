use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::MarketData;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningCriteria {
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub min_volume: Option<f64>,
    pub min_market_cap: Option<f64>,
    pub max_market_cap: Option<f64>,
    pub min_change_percent: Option<f64>,
    pub max_change_percent: Option<f64>,
    pub min_volume_ratio: Option<f64>,
    pub sectors: Vec<String>,
    pub exclude_sectors: Vec<String>,
    pub momentum_timeframes: Vec<MomentumTimeframe>,
    pub volatility_criteria: Option<VolatilityCriteria>,
    pub technical_indicators: Vec<TechnicalCriteria>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MomentumTimeframe {
    Minutes5,
    Minutes15,
    Minutes30,
    Hour1,
    Hour4,
    Day1,
    Week1,
    Month1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilityCriteria {
    pub min_volatility: f64,
    pub max_volatility: f64,
    pub lookback_periods: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TechnicalCriteria {
    RSI { min: f64, max: f64, period: usize },
    MACD { signal: MACDSignal },
    BollingerBands { position: BollingerPosition },
    VolumeSpike { threshold: f64 },
    PriceBreakout { resistance_level: f64 },
    MovingAverageCrossover { fast_period: usize, slow_period: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MACDSignal {
    BullishCrossover,
    BearishCrossover,
    AboveZero,
    BelowZero,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BollingerPosition {
    NearUpperBand,
    NearLowerBand,
    BandSqueeze,
    BandExpansion,
}

#[derive(Debug, Clone)]
pub struct StockScreener {
    criteria: ScreeningCriteria,
    market_history: HashMap<String, Vec<MarketData>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningResult {
    pub symbol: String,
    pub score: f64,
    pub reasons: Vec<String>,
    pub metrics: ScreeningMetrics,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningMetrics {
    pub momentum_score: f64,
    pub volume_score: f64,
    pub volatility_score: f64,
    pub technical_score: f64,
    pub fundamentals_score: f64,
    pub risk_score: f64,
}

impl Default for ScreeningCriteria {
    fn default() -> Self {
        Self {
            min_price: Some(5.0),
            max_price: Some(500.0),
            min_volume: Some(100000.0),
            min_market_cap: Some(100_000_000.0),
            max_market_cap: None,
            min_change_percent: None,
            max_change_percent: None,
            min_volume_ratio: Some(1.5),
            sectors: Vec::new(),
            exclude_sectors: vec!["Penny Stocks".to_string(), "OTC".to_string()],
            momentum_timeframes: vec![
                MomentumTimeframe::Minutes15,
                MomentumTimeframe::Hour1,
                MomentumTimeframe::Day1,
            ],
            volatility_criteria: Some(VolatilityCriteria {
                min_volatility: 0.5,
                max_volatility: 10.0,
                lookback_periods: 20,
            }),
            technical_indicators: vec![
                TechnicalCriteria::VolumeSpike { threshold: 2.0 },
                TechnicalCriteria::RSI { min: 30.0, max: 70.0, period: 14 },
            ],
        }
    }
}

impl StockScreener {
    pub fn new() -> Self {
        Self {
            criteria: ScreeningCriteria::default(),
            market_history: HashMap::new(),
        }
    }

    pub fn with_criteria(mut self, criteria: ScreeningCriteria) -> Self {
        self.criteria = criteria;
        self
    }

    pub async fn screen_symbols(&self, market_data: Vec<MarketData>) -> Result<Vec<MarketData>> {
        let mut filtered_symbols = Vec::new();

        for data in market_data {
            if self.passes_basic_filters(&data).await? {
                if let Ok(score) = self.calculate_screening_score(&data).await {
                    if score > 0.6 {
                        filtered_symbols.push(data);
                    }
                }
            }
        }

        filtered_symbols.sort_by(|a, b| {
            let score_a = self.calculate_screening_score(a).unwrap_or(0.0);
            let score_b = self.calculate_screening_score(b).unwrap_or(0.0);
            score_b.partial_cmp(&score_a).unwrap()
        });

        Ok(filtered_symbols)
    }

    pub async fn get_top_movers(&self, market_data: Vec<MarketData>, limit: usize) -> Result<Vec<ScreeningResult>> {
        let mut results = Vec::new();

        for data in market_data {
            if let Ok(result) = self.evaluate_symbol(&data).await {
                results.push(result);
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        Ok(results)
    }

    pub async fn scan_for_breakouts(&self, market_data: Vec<MarketData>) -> Result<Vec<ScreeningResult>> {
        let mut breakouts = Vec::new();

        for data in market_data {
            if self.is_breakout_candidate(&data).await? {
                if let Ok(result) = self.evaluate_symbol(&data).await {
                    breakouts.push(result);
                }
            }
        }

        breakouts.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(breakouts)
    }

    pub async fn scan_for_momentum(&self, market_data: Vec<MarketData>) -> Result<Vec<ScreeningResult>> {
        let mut momentum_plays = Vec::new();

        for data in market_data {
            if self.has_strong_momentum(&data).await? {
                if let Ok(result) = self.evaluate_symbol(&data).await {
                    momentum_plays.push(result);
                }
            }
        }

        momentum_plays.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(momentum_plays)
    }

    async fn passes_basic_filters(&self, data: &MarketData) -> Result<bool> {
        if let Some(min_price) = self.criteria.min_price {
            if data.price < min_price {
                return Ok(false);
            }
        }

        if let Some(max_price) = self.criteria.max_price {
            if data.price > max_price {
                return Ok(false);
            }
        }

        if let Some(min_volume) = self.criteria.min_volume {
            if data.volume < min_volume {
                return Ok(false);
            }
        }

        if let Some(min_change) = self.criteria.min_change_percent {
            if data.change_24h.abs() < min_change {
                return Ok(false);
            }
        }

        if let Some(max_change) = self.criteria.max_change_percent {
            if data.change_24h.abs() > max_change {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn calculate_screening_score(&self, data: &MarketData) -> Result<f64> {
        let mut total_score = 0.0;
        let mut criteria_count = 0;

        total_score += self.calculate_momentum_score(data).await? * 0.3;
        total_score += self.calculate_volume_score(data).await? * 0.25;
        total_score += self.calculate_volatility_score(data).await? * 0.2;
        total_score += self.calculate_technical_score(data).await? * 0.25;

        criteria_count = 4;

        Ok(total_score / criteria_count as f64)
    }

    async fn calculate_momentum_score(&self, data: &MarketData) -> Result<f64> {
        let change_magnitude = data.change_24h.abs();
        
        let score = match change_magnitude {
            x if x > 10.0 => 1.0,
            x if x > 5.0 => 0.8,
            x if x > 2.0 => 0.6,
            x if x > 1.0 => 0.4,
            _ => 0.2,
        };

        Ok(score)
    }

    async fn calculate_volume_score(&self, data: &MarketData) -> Result<f64> {
        if let Some(min_volume) = self.criteria.min_volume {
            let volume_ratio = data.volume / min_volume;
            let score = match volume_ratio {
                x if x > 5.0 => 1.0,
                x if x > 3.0 => 0.8,
                x if x > 2.0 => 0.6,
                x if x > 1.0 => 0.4,
                _ => 0.2,
            };
            Ok(score)
        } else {
            Ok(0.5)
        }
    }

    async fn calculate_volatility_score(&self, data: &MarketData) -> Result<f64> {
        let price_range = (data.high - data.low) / data.price;
        
        let score = match price_range {
            x if x > 0.1 => 1.0,
            x if x > 0.05 => 0.8,
            x if x > 0.03 => 0.6,
            x if x > 0.01 => 0.4,
            _ => 0.2,
        };

        Ok(score)
    }

    async fn calculate_technical_score(&self, data: &MarketData) -> Result<f64> {
        let mut score = 0.5;

        if data.price > (data.high + data.low) / 2.0 {
            score += 0.2;
        }

        if data.volume > data.volume_24h * 1.5 {
            score += 0.3;
        }

        Ok(score.min(1.0))
    }

    async fn is_breakout_candidate(&self, data: &MarketData) -> Result<bool> {
        let price_near_high = (data.price - data.high).abs() / data.price < 0.02;
        let volume_spike = data.volume > data.volume_24h * 1.5;
        let momentum = data.change_24h > 3.0;

        Ok(price_near_high && volume_spike && momentum)
    }

    async fn has_strong_momentum(&self, data: &MarketData) -> Result<bool> {
        let strong_change = data.change_24h.abs() > 5.0;
        let volume_support = data.volume > data.volume_24h * 1.2;
        
        Ok(strong_change && volume_support)
    }

    async fn evaluate_symbol(&self, data: &MarketData) -> Result<ScreeningResult> {
        let momentum_score = self.calculate_momentum_score(data).await?;
        let volume_score = self.calculate_volume_score(data).await?;
        let volatility_score = self.calculate_volatility_score(data).await?;
        let technical_score = self.calculate_technical_score(data).await?;

        let overall_score = (momentum_score + volume_score + volatility_score + technical_score) / 4.0;

        let mut reasons = Vec::new();
        if momentum_score > 0.7 {
            reasons.push(format!("Strong momentum: {:.1}% change", data.change_24h));
        }
        if volume_score > 0.7 {
            reasons.push("High volume activity".to_string());
        }
        if volatility_score > 0.7 {
            reasons.push("High intraday volatility".to_string());
        }

        Ok(ScreeningResult {
            symbol: data.symbol.as_str().to_string(),
            score: overall_score,
            reasons,
            metrics: ScreeningMetrics {
                momentum_score,
                volume_score,
                volatility_score,
                technical_score,
                fundamentals_score: 0.5,
                risk_score: 1.0 - overall_score,
            },
            timestamp: Utc::now(),
        })
    }
}