use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{MarketData, MarketMetrics, MarketRegime};
use crate::exchanges::Symbol;

#[derive(Clone)]
pub struct MarketAnalytics {
    sector_classifications: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicators {
    pub rsi: Option<f64>,
    pub macd: Option<MACDValues>,
    pub bollinger_bands: Option<BollingerBands>,
    pub moving_averages: MovingAverages,
    pub volume_indicators: VolumeIndicators,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MACDValues {
    pub macd_line: f64,
    pub signal_line: f64,
    pub histogram: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BollingerBands {
    pub upper_band: f64,
    pub middle_band: f64,
    pub lower_band: f64,
    pub bandwidth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovingAverages {
    pub sma_20: Option<f64>,
    pub sma_50: Option<f64>,
    pub sma_200: Option<f64>,
    pub ema_12: Option<f64>,
    pub ema_26: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeIndicators {
    pub volume_sma: Option<f64>,
    pub volume_ratio: f64,
    pub on_balance_volume: Option<f64>,
    pub volume_weighted_price: f64,
}

impl MarketAnalytics {
    pub fn new() -> Self {
        let mut sector_classifications = HashMap::new();
        
        sector_classifications.insert("AAPL".to_string(), "Technology".to_string());
        sector_classifications.insert("MSFT".to_string(), "Technology".to_string());
        sector_classifications.insert("GOOGL".to_string(), "Technology".to_string());
        sector_classifications.insert("AMZN".to_string(), "Consumer Discretionary".to_string());
        sector_classifications.insert("TSLA".to_string(), "Consumer Discretionary".to_string());
        sector_classifications.insert("META".to_string(), "Technology".to_string());
        sector_classifications.insert("NVDA".to_string(), "Technology".to_string());
        sector_classifications.insert("JPM".to_string(), "Financials".to_string());
        sector_classifications.insert("JNJ".to_string(), "Healthcare".to_string());
        
        Self {
            sector_classifications,
        }
    }

    pub async fn calculate_market_metrics(&self, market_data: Vec<MarketData>) -> Result<MarketMetrics> {
        let total_symbols = market_data.len();
        let opportunities_detected = self.count_opportunities(&market_data).await;
        let market_volatility = self.calculate_market_volatility(&market_data);
        let sector_performance = self.calculate_sector_performance(&market_data);
        let trending_symbols = self.identify_trending_symbols(&market_data);
        let market_regime = self.determine_market_regime(&market_data);
        let overall_sentiment = self.calculate_market_sentiment(&market_data);

        Ok(MarketMetrics {
            total_symbols_tracked: total_symbols,
            opportunities_detected,
            market_volatility,
            sector_performance,
            trending_symbols,
            market_regime,
            overall_sentiment,
        })
    }

    pub async fn calculate_technical_indicators(&self, symbol: &str, history: &[MarketData]) -> Result<TechnicalIndicators> {
        let rsi = self.calculate_rsi(history, 14);
        let macd = self.calculate_macd(history);
        let bollinger_bands = self.calculate_bollinger_bands(history, 20, 2.0);
        let moving_averages = self.calculate_moving_averages(history);
        let volume_indicators = self.calculate_volume_indicators(history);

        Ok(TechnicalIndicators {
            rsi,
            macd,
            bollinger_bands,
            moving_averages,
            volume_indicators,
        })
    }

    pub fn calculate_correlation(&self, data1: &[MarketData], data2: &[MarketData]) -> f64 {
        if data1.len() != data2.len() || data1.is_empty() {
            return 0.0;
        }

        let prices1: Vec<f64> = data1.iter().map(|d| d.price).collect();
        let prices2: Vec<f64> = data2.iter().map(|d| d.price).collect();

        self.pearson_correlation(&prices1, &prices2)
    }

    pub fn detect_patterns(&self, history: &[MarketData]) -> Vec<String> {
        let mut patterns = Vec::new();

        if self.is_ascending_triangle(history) {
            patterns.push("Ascending Triangle".to_string());
        }

        if self.is_head_and_shoulders(history) {
            patterns.push("Head and Shoulders".to_string());
        }

        if self.is_double_bottom(history) {
            patterns.push("Double Bottom".to_string());
        }

        if self.is_flag_pattern(history) {
            patterns.push("Flag Pattern".to_string());
        }

        patterns
    }

    async fn count_opportunities(&self, market_data: &[MarketData]) -> usize {
        market_data.iter()
            .filter(|data| {
                data.change_24h.abs() > 3.0 && 
                data.volume > 500000.0 &&
                (data.price - data.low) / (data.high - data.low) > 0.7
            })
            .count()
    }

    fn calculate_market_volatility(&self, market_data: &[MarketData]) -> f64 {
        if market_data.is_empty() {
            return 0.0;
        }

        let volatilities: Vec<f64> = market_data.iter()
            .map(|data| (data.high - data.low) / data.price)
            .collect();

        volatilities.iter().sum::<f64>() / volatilities.len() as f64
    }

    fn calculate_sector_performance(&self, market_data: &[MarketData]) -> HashMap<String, f64> {
        let mut sector_performance = HashMap::new();
        let mut sector_counts = HashMap::new();

        for data in market_data {
            if let Some(sector) = self.sector_classifications.get(data.symbol.as_str()) {
                let current_perf = sector_performance.get(sector).unwrap_or(&0.0);
                let count = sector_counts.get(sector).unwrap_or(&0);
                
                sector_performance.insert(sector.clone(), current_perf + data.change_24h);
                sector_counts.insert(sector.clone(), count + 1);
            }
        }

        for (sector, total_change) in sector_performance.iter_mut() {
            if let Some(count) = sector_counts.get(sector) {
                if *count > 0 {
                    *total_change /= *count as f64;
                }
            }
        }

        sector_performance
    }

    fn identify_trending_symbols(&self, market_data: &[MarketData]) -> Vec<Symbol> {
        let mut trending = market_data.iter()
            .filter(|data| data.change_24h.abs() > 5.0 && data.volume > 1000000.0)
            .map(|data| data.symbol.clone())
            .collect::<Vec<_>>();

        trending.sort_by(|a, b| {
            let data_a = market_data.iter().find(|d| d.symbol == *a).unwrap();
            let data_b = market_data.iter().find(|d| d.symbol == *b).unwrap();
            data_b.change_24h.abs().partial_cmp(&data_a.change_24h.abs()).unwrap()
        });

        trending.truncate(10);
        trending
    }

    fn determine_market_regime(&self, market_data: &[MarketData]) -> MarketRegime {
        if market_data.is_empty() {
            return MarketRegime::Consolidation;
        }

        let avg_change = market_data.iter()
            .map(|d| d.change_24h)
            .sum::<f64>() / market_data.len() as f64;

        let volatility = self.calculate_market_volatility(market_data);

        match (avg_change, volatility) {
            (change, vol) if change > 2.0 && vol < 0.03 => MarketRegime::StrongBull,
            (change, vol) if change > 0.5 && vol < 0.05 => MarketRegime::MildBull,
            (change, vol) if change < -2.0 && vol < 0.03 => MarketRegime::StrongBear,
            (change, vol) if change < -0.5 && vol < 0.05 => MarketRegime::MildBear,
            (_, vol) if vol > 0.08 => MarketRegime::HighVolatility,
            (_, vol) if vol < 0.02 => MarketRegime::LowVolatility,
            _ => MarketRegime::Consolidation,
        }
    }

    fn calculate_market_sentiment(&self, market_data: &[MarketData]) -> f64 {
        if market_data.is_empty() {
            return 0.0;
        }

        let positive_moves = market_data.iter()
            .filter(|d| d.change_24h > 0.0)
            .count() as f64;

        let total_moves = market_data.len() as f64;

        (positive_moves / total_moves - 0.5) * 2.0
    }

    fn calculate_rsi(&self, history: &[MarketData], period: usize) -> Option<f64> {
        if history.len() < period + 1 {
            return None;
        }

        let mut gains = Vec::new();
        let mut losses = Vec::new();

        for i in 1..history.len() {
            let change = history[i].price - history[i-1].price;
            if change > 0.0 {
                gains.push(change);
                losses.push(0.0);
            } else {
                gains.push(0.0);
                losses.push(-change);
            }
        }

        if gains.len() < period {
            return None;
        }

        let avg_gain = gains[gains.len()-period..].iter().sum::<f64>() / period as f64;
        let avg_loss = losses[losses.len()-period..].iter().sum::<f64>() / period as f64;

        if avg_loss == 0.0 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        Some(100.0 - (100.0 / (1.0 + rs)))
    }

    fn calculate_macd(&self, history: &[MarketData]) -> Option<MACDValues> {
        if history.len() < 26 {
            return None;
        }

        let ema12 = self.calculate_ema(history, 12)?;
        let ema26 = self.calculate_ema(history, 26)?;
        let macd_line = ema12 - ema26;

        let signal_line = 0.0;
        let histogram = macd_line - signal_line;

        Some(MACDValues {
            macd_line,
            signal_line,
            histogram,
        })
    }

    fn calculate_ema(&self, history: &[MarketData], period: usize) -> Option<f64> {
        if history.len() < period {
            return None;
        }

        let multiplier = 2.0 / (period + 1) as f64;
        let mut ema = history[0].price;

        for data in &history[1..] {
            ema = (data.price - ema) * multiplier + ema;
        }

        Some(ema)
    }

    fn calculate_bollinger_bands(&self, history: &[MarketData], period: usize, std_dev: f64) -> Option<BollingerBands> {
        if history.len() < period {
            return None;
        }

        let recent_prices: Vec<f64> = history[history.len()-period..]
            .iter()
            .map(|d| d.price)
            .collect();

        let middle_band = recent_prices.iter().sum::<f64>() / period as f64;
        
        let variance = recent_prices.iter()
            .map(|price| (price - middle_band).powi(2))
            .sum::<f64>() / period as f64;
        
        let std_deviation = variance.sqrt();
        
        let upper_band = middle_band + (std_deviation * std_dev);
        let lower_band = middle_band - (std_deviation * std_dev);
        let bandwidth = (upper_band - lower_band) / middle_band;

        Some(BollingerBands {
            upper_band,
            middle_band,
            lower_band,
            bandwidth,
        })
    }

    fn calculate_moving_averages(&self, history: &[MarketData]) -> MovingAverages {
        MovingAverages {
            sma_20: self.calculate_sma(history, 20),
            sma_50: self.calculate_sma(history, 50),
            sma_200: self.calculate_sma(history, 200),
            ema_12: self.calculate_ema(history, 12),
            ema_26: self.calculate_ema(history, 26),
        }
    }

    fn calculate_sma(&self, history: &[MarketData], period: usize) -> Option<f64> {
        if history.len() < period {
            return None;
        }

        let sum = history[history.len()-period..]
            .iter()
            .map(|d| d.price)
            .sum::<f64>();

        Some(sum / period as f64)
    }

    fn calculate_volume_indicators(&self, history: &[MarketData]) -> VolumeIndicators {
        let volume_sma = if history.len() >= 20 {
            let vol_sum = history[history.len()-20..]
                .iter()
                .map(|d| d.volume)
                .sum::<f64>();
            Some(vol_sum / 20.0)
        } else {
            None
        };

        let current_volume = history.last().map(|d| d.volume).unwrap_or(0.0);
        let volume_ratio = if let Some(avg_vol) = volume_sma {
            current_volume / avg_vol
        } else {
            1.0
        };

        let volume_weighted_price = if !history.is_empty() {
            let last_data = &history[history.len()-1];
            (last_data.high + last_data.low + last_data.price) / 3.0
        } else {
            0.0
        };

        VolumeIndicators {
            volume_sma,
            volume_ratio,
            on_balance_volume: None,
            volume_weighted_price,
        }
    }

    fn pearson_correlation(&self, x: &[f64], y: &[f64]) -> f64 {
        if x.len() != y.len() || x.is_empty() {
            return 0.0;
        }

        let n = x.len() as f64;
        let sum_x = x.iter().sum::<f64>();
        let sum_y = y.iter().sum::<f64>();
        let sum_xy = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum::<f64>();
        let sum_x2 = x.iter().map(|a| a * a).sum::<f64>();
        let sum_y2 = y.iter().map(|b| b * b).sum::<f64>();

        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    fn is_ascending_triangle(&self, history: &[MarketData]) -> bool {
        history.len() > 10
    }

    fn is_head_and_shoulders(&self, history: &[MarketData]) -> bool {
        history.len() > 15
    }

    fn is_double_bottom(&self, history: &[MarketData]) -> bool {
        history.len() > 10
    }

    fn is_flag_pattern(&self, history: &[MarketData]) -> bool {
        history.len() > 8
    }
}